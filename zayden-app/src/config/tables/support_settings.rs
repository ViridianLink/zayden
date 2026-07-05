use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SupportSettingsRow {
    pub guild_id: i64,
    pub support_channel_id: Option<i64>,
    pub support_thread_id: i32,
    pub support_role_id: Option<i64>,
    pub faq_channel_id: Option<i64>,
}

impl SettingsRow for SupportSettingsRow {
    const TABLE: &'static str = "support_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            support_channel_id: None,
            support_thread_id: 0,
            support_role_id: None,
            faq_channel_id: None,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, support_channel_id, support_thread_id, support_role_id, faq_channel_id
            FROM support_settings
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
            INSERT INTO support_settings (guild_id, support_channel_id, support_thread_id, support_role_id, faq_channel_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (guild_id) DO UPDATE SET
                support_channel_id = EXCLUDED.support_channel_id,
                support_thread_id = EXCLUDED.support_thread_id,
                support_role_id = EXCLUDED.support_role_id,
                faq_channel_id = EXCLUDED.faq_channel_id,
                updated_at = now()
            RETURNING guild_id, support_channel_id, support_thread_id, support_role_id, faq_channel_id
            "#,
            self.guild_id,
            self.support_channel_id,
            self.support_thread_id,
            self.support_role_id,
            self.faq_channel_id
        )
        .fetch_one(pool)
        .await
    }
}
