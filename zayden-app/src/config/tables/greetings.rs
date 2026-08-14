use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GreetingsSettingsRow {
    pub guild_id: i64,
    pub morning_message: Option<String>,
    pub night_message: Option<String>,
}

impl SettingsRow for GreetingsSettingsRow {
    const TABLE: &'static str = "greetings_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, morning_message: None, night_message: None }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT
                guild_id,
                morning_message,
                night_message
            FROM greetings_settings
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
            INSERT INTO greetings_settings (
                guild_id, morning_message, night_message
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                morning_message = EXCLUDED.morning_message,
                night_message = EXCLUDED.night_message,
                updated_at = now()
            RETURNING
                guild_id,
                morning_message,
                night_message
            "#,
            self.guild_id,
            self.morning_message.as_deref(),
            self.night_message.as_deref()
        )
        .fetch_one(pool)
        .await
    }
}
