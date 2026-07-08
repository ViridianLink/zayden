use sqlx::PgPool;

use crate::error::Result;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MarathonAnnounceRow {
    pub guild_id: i64,
    pub channel_id: i64,
    pub last_rotation: Option<String>,
}

impl MarathonAnnounceRow {
    pub async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            "SELECT guild_id, channel_id, last_rotation FROM marathon_announce WHERE guild_id = $1",
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert(
        pool: &PgPool,
        guild_id: i64,
        channel_id: i64,
    ) -> Result<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"
            INSERT INTO marathon_announce (guild_id, channel_id)
            VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE SET channel_id = EXCLUDED.channel_id
            RETURNING guild_id, channel_id, last_rotation
            "#,
            guild_id,
            channel_id
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM marathon_announce WHERE guild_id = $1", guild_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn all(pool: &PgPool) -> Result<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            "SELECT guild_id, channel_id, last_rotation FROM marathon_announce"
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn set_last_rotation(
        pool: &PgPool,
        guild_id: i64,
        last_rotation: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE marathon_announce SET last_rotation = $2 WHERE guild_id = $1",
            guild_id,
            last_rotation
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NewsSeenRow {
    pub source: String,
    pub last_id: Option<String>,
}

impl NewsSeenRow {
    pub async fn get(pool: &PgPool, source: &str) -> Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            "SELECT source, last_id FROM marathon_news_seen WHERE source = $1",
            source
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn set_last_id(
        pool: &PgPool,
        source: &str,
        last_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO marathon_news_seen (source, last_id)
            VALUES ($1, $2)
            ON CONFLICT (source) DO UPDATE SET last_id = EXCLUDED.last_id, updated_at = now()
            "#,
            source,
            last_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
