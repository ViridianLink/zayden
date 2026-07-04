use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildChannelsRow {
    pub guild_id: i64,
    pub rules_channel_id: Option<i64>,
    pub general_channel_id: Option<i64>,
    pub spoiler_channel_id: Option<i64>,
}

impl SettingsRow for GuildChannelsRow {
    const TABLE: &'static str = "guild_channels";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            rules_channel_id: None,
            general_channel_id: None,
            spoiler_channel_id: None,
        }
    }

    async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, rules_channel_id, general_channel_id, spoiler_channel_id
            FROM guild_channels
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
            INSERT INTO guild_channels (guild_id, rules_channel_id, general_channel_id, spoiler_channel_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (guild_id) DO UPDATE SET
                rules_channel_id = EXCLUDED.rules_channel_id,
                general_channel_id = EXCLUDED.general_channel_id,
                spoiler_channel_id = EXCLUDED.spoiler_channel_id,
                updated_at = now()
            RETURNING guild_id, rules_channel_id, general_channel_id, spoiler_channel_id
            "#,
            self.guild_id,
            self.rules_channel_id,
            self.general_channel_id,
            self.spoiler_channel_id
        )
        .fetch_one(pool)
        .await
    }
}
