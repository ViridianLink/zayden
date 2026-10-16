use std::sync::Arc;

use deadlock_api::DeadlockApiError;
use moka::future::Cache;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

use super::{CachePolicy, DeadlockApi, Fetched};
use crate::error::{DeadlockError, Result};

struct CacheRow {
    body: String,
    fetched_at: jiff_sqlx::Timestamp,
    fresh: bool,
}

fn from_row<T: DeserializeOwned>(
    key: &str,
    row: &CacheRow,
    stale: bool,
) -> Result<Fetched<T>> {
    let value = serde_json::from_str(&row.body).map_err(|source| {
        DeadlockError::CacheDecode { key: key.to_owned(), source }
    })?;

    Ok(Fetched {
        value: Arc::new(value),
        fetched_at: row.fetched_at.to_jiff(),
        stale,
    })
}

impl DeadlockApi {
    pub(super) async fn cached<T, F, Fut>(
        &self,
        l1: &Cache<String, Fetched<T>>,
        key: String,
        policy: CachePolicy,
        fetch: F,
    ) -> Result<Fetched<T>>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = deadlock_api::Result<T>>,
    {
        let key = policy.client_version.map_or_else(
            || format!("{key}#latest"),
            |version| format!("{key}#v{version}"),
        );

        if let Some(hit) = l1.get(&key).await {
            return Ok(hit);
        }

        let row = self.read_row(&key).await?;

        if let Some(row) = &row
            && row.fresh
        {
            let fetched = from_row(&key, row, false)?;
            l1.insert(key, fetched.clone()).await;
            return Ok(fetched);
        }

        let error = match fetch().await {
            Ok(value) => {
                match serde_json::to_string(&value) {
                    Ok(body) => {
                        if let Err(error) = self.write_row(&key, policy, &body).await
                        {
                            warn!(
                                %error,
                                %key,
                                "failed to write deadlock-api cache row"
                            );
                        }
                    },
                    Err(error) => {
                        warn!(
                            %error,
                            %key,
                            "failed to serialize deadlock-api cache row"
                        );
                    },
                }

                let fetched = Fetched {
                    value: Arc::new(value),
                    fetched_at: jiff::Timestamp::now(),
                    stale: false,
                };

                l1.insert(key, fetched.clone()).await;
                return Ok(fetched);
            },
            Err(error) => error,
        };

        if matches!(
            error,
            DeadlockApiError::Busy { .. }
                | DeadlockApiError::Http { .. }
                | DeadlockApiError::Request(_)
        ) && let Some(row) = &row
        {
            warn!(%error, %key, "deadlock-api unavailable, serving stale cache row");
            return from_row(&key, row, true);
        }

        Err(error.into())
    }

    async fn read_row(&self, key: &str) -> Result<Option<CacheRow>> {
        let row = sqlx::query_as!(
            CacheRow,
            r#"SELECT
                body,
                fetched_at as "fetched_at: jiff_sqlx::Timestamp",
                expires_at > now() AS "fresh!"
            FROM deadlock_api_cache
            WHERE key = $1"#,
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn write_row(
        &self,
        key: &str,
        policy: CachePolicy,
        body: &str,
    ) -> Result<()> {
        let client_version =
            policy.client_version.and_then(|v| i32::try_from(v).ok());

        sqlx::query!(
            "INSERT INTO deadlock_api_cache
                (key, kind, client_version, body, fetched_at, expires_at)
            VALUES ($1, $2, $3, $4, now(), now() + make_interval(secs => $5))
            ON CONFLICT (key) DO UPDATE SET
                kind = EXCLUDED.kind,
                client_version = EXCLUDED.client_version,
                body = EXCLUDED.body,
                fetched_at = now(),
                expires_at = EXCLUDED.expires_at",
            key,
            policy.kind.as_str(),
            client_version,
            body,
            policy.ttl.as_secs_f64()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
