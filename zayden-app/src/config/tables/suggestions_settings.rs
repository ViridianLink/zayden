use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SuggestionsSettingsRow {
    pub guild_id: i64,
    pub suggestions_channel_id: Option<i64>,
    pub review_channel_id: Option<i64>,
}

impl SettingsRow for SuggestionsSettingsRow {
    const TABLE: &'static str = "suggestions_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, suggestions_channel_id: None, review_channel_id: None }
    }

    async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, suggestions_channel_id, review_channel_id
            FROM suggestions_settings
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
            INSERT INTO suggestions_settings (guild_id, suggestions_channel_id, review_channel_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                suggestions_channel_id = EXCLUDED.suggestions_channel_id,
                review_channel_id = EXCLUDED.review_channel_id,
                updated_at = now()
            RETURNING guild_id, suggestions_channel_id, review_channel_id
            "#,
            self.guild_id,
            self.suggestions_channel_id,
            self.review_channel_id
        )
        .fetch_one(pool)
        .await
    }
}
