use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiSettingsRow {
    pub guild_id: i64,
    pub enabled: bool,
    pub channel_id: Option<i64>,
}

impl AiSettingsRow {
    #[must_use]
    pub fn responds_in(&self, channel_id: i64) -> bool {
        self.enabled && self.channel_id.is_none_or(|only| only == channel_id)
    }
}

impl SettingsRow for AiSettingsRow {
    const TABLE: &'static str = "ai_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, enabled: false, channel_id: None }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, enabled, channel_id
            FROM ai_settings
            WHERE guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await
    }

    async fn upsert(&self, pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO ai_settings (guild_id, enabled, channel_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                channel_id = EXCLUDED.channel_id,
                updated_at = now()
            RETURNING guild_id, enabled, channel_id
            "#,
            self.guild_id,
            self.enabled,
            self.channel_id
        )
        .fetch_one(pool)
        .await
    }
}
