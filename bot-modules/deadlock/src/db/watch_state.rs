use sqlx::PgPool;

use crate::error::{DeadlockError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchKey {
    Latest,
    Baseline,
}

impl WatchKey {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Latest => "latest",
            Self::Baseline => "baseline",
        }
    }
}

pub(super) fn column_version(version: u32) -> Result<i32> {
    let Ok(column) = i32::try_from(version) else {
        return Err(DeadlockError::InvalidOption(format!(
            "client version {version} is out of range"
        )));
    };

    Ok(column)
}

pub async fn get(pool: &PgPool, key: WatchKey) -> sqlx::Result<Option<u32>> {
    let version = sqlx::query_scalar!(
        "SELECT client_version FROM deadlock_watch_state WHERE key = $1",
        key.as_str()
    )
    .fetch_optional(pool)
    .await?;

    Ok(version.and_then(|version| u32::try_from(version).ok()))
}

pub async fn set(pool: &PgPool, key: WatchKey, version: u32) -> Result<()> {
    let version = column_version(version)?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_watch_state (key, client_version)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE
        SET client_version = EXCLUDED.client_version,
            updated_at = now()
        "#,
        key.as_str(),
        version
    )
    .execute(pool)
    .await?;

    Ok(())
}
