use jiff::{SignedDuration, Timestamp};
use jiff_sqlx::Timestamp as SqlxTimestamp;
use sqlx::PgPool;
use zayden_app::entitlement::Tier;

use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub struct UploadQuota {
    pub cooldown: SignedDuration,
    pub max_bytes: u64,
}

impl UploadQuota {
    pub const FREE: Self = Self {
        cooldown: SignedDuration::from_mins(60),
        max_bytes: 30 * 1024 * 1024,
    };
    pub const PRO: Self = Self {
        cooldown: SignedDuration::from_mins(30),
        max_bytes: 100 * 1024 * 1024,
    };

    #[must_use]
    pub const fn for_tier(tier: Tier) -> Self {
        match tier {
            Tier::Free => Self::FREE,
            Tier::Pro | Tier::Ultra => Self::PRO,
        }
    }

    #[must_use]
    pub const fn max_megabytes(&self) -> u64 {
        self.max_bytes / (1024 * 1024)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SaveUpload {
    pub discord_id: i64,
    pub file_path: String,
    pub uploaded_at: SqlxTimestamp,
    pub expires_at: SqlxTimestamp,
}

impl SaveUpload {
    #[must_use]
    pub fn cooldown_remaining(
        &self,
        cooldown: SignedDuration,
    ) -> Option<SignedDuration> {
        let deadline = self.uploaded_at.to_jiff().checked_add(cooldown).ok()?;
        let now = Timestamp::now();
        (deadline > now).then(|| deadline.duration_since(now))
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.to_jiff() <= Timestamp::now()
    }

    pub async fn select(pool: &PgPool, discord_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT
                discord_id,
                file_path,
                uploaded_at AS "uploaded_at: SqlxTimestamp",
                expires_at AS "expires_at: SqlxTimestamp"
            FROM palworld_save_uploads
            WHERE discord_id = $1
            "#,
            discord_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert(
        pool: &PgPool,
        discord_id: i64,
        file_path: &str,
    ) -> Result<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"
            INSERT INTO palworld_save_uploads (discord_id, file_path, uploaded_at, expires_at)
            VALUES ($1, $2, now(), now() + interval '7 days')
            ON CONFLICT (discord_id) DO UPDATE
                SET file_path = EXCLUDED.file_path,
                    uploaded_at = EXCLUDED.uploaded_at,
                    expires_at = EXCLUDED.expires_at
            RETURNING
                discord_id,
                file_path,
                uploaded_at AS "uploaded_at: SqlxTimestamp",
                expires_at AS "expires_at: SqlxTimestamp"
            "#,
            discord_id,
            file_path
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn delete(pool: &PgPool, discord_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM palworld_save_uploads WHERE discord_id = $1",
            discord_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_expired(pool: &PgPool) -> Result<Vec<String>> {
        let paths = sqlx::query_scalar!(
            "DELETE FROM palworld_save_uploads WHERE expires_at < now() RETURNING file_path"
        )
        .fetch_all(pool)
        .await?;

        Ok(paths)
    }
}
