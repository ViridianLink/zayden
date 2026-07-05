use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LfgSettingsRow {
    pub guild_id: i64,
    pub lfg_channel_id: Option<i64>,
    pub lfg_role_id: Option<i64>,
    pub lfg_scheduled_thread_id: Option<i64>,
}

impl SettingsRow for LfgSettingsRow {
    const TABLE: &'static str = "lfg_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            lfg_channel_id: None,
            lfg_role_id: None,
            lfg_scheduled_thread_id: None,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, lfg_channel_id, lfg_role_id, lfg_scheduled_thread_id
            FROM lfg_settings
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
            INSERT INTO lfg_settings (guild_id, lfg_channel_id, lfg_role_id, lfg_scheduled_thread_id)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (guild_id) DO UPDATE SET
                lfg_channel_id = EXCLUDED.lfg_channel_id,
                lfg_role_id = EXCLUDED.lfg_role_id,
                lfg_scheduled_thread_id = EXCLUDED.lfg_scheduled_thread_id,
                updated_at = now()
            RETURNING guild_id, lfg_channel_id, lfg_role_id, lfg_scheduled_thread_id
            "#,
            self.guild_id,
            self.lfg_channel_id,
            self.lfg_role_id,
            self.lfg_scheduled_thread_id
        )
        .fetch_one(pool)
        .await
    }
}
