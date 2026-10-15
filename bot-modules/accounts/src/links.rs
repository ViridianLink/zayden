use sqlx::PgPool;

use crate::error::Result;
use crate::provider::Provider;
use crate::steam::SteamId64;

#[derive(Debug, Clone)]
pub struct AccountLink {
    pub discord_id: i64,
    pub provider: Provider,
    pub external_id: String,
    pub linked_at: jiff::Timestamp,
}

struct LinkRow {
    discord_id: i64,
    provider: String,
    external_id: String,
    linked_at: jiff_sqlx::Timestamp,
}

impl LinkRow {
    fn into_link(self) -> Option<AccountLink> {
        match self.provider.parse() {
            Ok(provider) => Some(AccountLink {
                discord_id: self.discord_id,
                provider,
                external_id: self.external_id,
                linked_at: self.linked_at.to_jiff(),
            }),
            Err(error) => {
                tracing::warn!(%error, "skipping account link with unknown provider");
                None
            },
        }
    }
}

impl AccountLink {
    pub async fn select(
        pool: &PgPool,
        discord_id: i64,
        provider: Provider,
    ) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            LinkRow,
            r#"SELECT
                discord_id,
                provider,
                external_id,
                linked_at as "linked_at: jiff_sqlx::Timestamp"
            FROM account_links
            WHERE discord_id = $1 AND provider = $2"#,
            discord_id,
            provider.as_str()
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.and_then(LinkRow::into_link))
    }

    pub async fn list_for_user(pool: &PgPool, discord_id: i64) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            LinkRow,
            r#"SELECT
                discord_id,
                provider,
                external_id,
                linked_at as "linked_at: jiff_sqlx::Timestamp"
            FROM account_links
            WHERE discord_id = $1
            ORDER BY provider"#,
            discord_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().filter_map(LinkRow::into_link).collect())
    }

    pub async fn upsert_transfer(
        pool: &PgPool,
        discord_id: i64,
        provider: Provider,
        external_id: &str,
    ) -> Result<()> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "DELETE FROM account_links
            WHERE provider = $1 AND external_id = $2 AND discord_id <> $3",
            provider.as_str(),
            external_id,
            discord_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO account_links (discord_id, provider, external_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (discord_id, provider)
            DO UPDATE SET external_id = EXCLUDED.external_id, linked_at = now()",
            discord_id,
            provider.as_str(),
            external_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn delete(
        pool: &PgPool,
        discord_id: i64,
        provider: Provider,
    ) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM account_links WHERE discord_id = $1 AND provider = $2",
            discord_id,
            provider.as_str()
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    #[must_use]
    pub fn steam_id(&self) -> Option<SteamId64> {
        match self.provider {
            Provider::Steam => self.external_id.parse().ok(),
        }
    }
}
