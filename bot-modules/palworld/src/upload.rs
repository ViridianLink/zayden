use jiff::{SignedDuration, Timestamp};
use jiff_sqlx::Timestamp as SqlxTimestamp;
use sqlx::PgPool;

use crate::error::Result;

pub const UPLOAD_COOLDOWN: SignedDuration = SignedDuration::from_hours(1);

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SaveUpload {
    pub discord_id: i64,
    pub file_path: String,
    pub uploaded_at: SqlxTimestamp,
    pub expires_at: SqlxTimestamp,
}

impl SaveUpload {
    #[must_use]
    pub fn cooldown_remaining(&self) -> Option<SignedDuration> {
        let deadline =
            self.uploaded_at.to_jiff().checked_add(UPLOAD_COOLDOWN).ok()?;
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
