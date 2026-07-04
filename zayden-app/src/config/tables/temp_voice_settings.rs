use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TempVoiceSettingsRow {
    pub guild_id: i64,
    pub temp_voice_category: Option<i64>,
    pub temp_voice_creator_channel: Option<i64>,
}

impl SettingsRow for TempVoiceSettingsRow {
    const TABLE: &'static str = "temp_voice_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, temp_voice_category: None, temp_voice_creator_channel: None }
    }

    async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, temp_voice_category, temp_voice_creator_channel
            FROM temp_voice_settings
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
            INSERT INTO temp_voice_settings (guild_id, temp_voice_category, temp_voice_creator_channel)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                temp_voice_category = EXCLUDED.temp_voice_category,
                temp_voice_creator_channel = EXCLUDED.temp_voice_creator_channel,
                updated_at = now()
            RETURNING guild_id, temp_voice_category, temp_voice_creator_channel
            "#,
            self.guild_id,
            self.temp_voice_category,
            self.temp_voice_creator_channel
        )
        .fetch_one(pool)
        .await
    }
}
