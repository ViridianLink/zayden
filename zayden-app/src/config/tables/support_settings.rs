use sqlx::PgPool;

use crate::config::SettingsRow;

pub const ARCHIVE_NEVER: i32 = -1;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SupportSettingsRow {
    pub guild_id: i64,
    pub support_channel_id: Option<i64>,
    pub faq_channel_id: Option<i64>,
    pub solved_tag_id: Option<i64>,
    pub helper_role_id: Option<i64>,
    pub solved_archive_secs: i32,
}

impl SettingsRow for SupportSettingsRow {
    const TABLE: &'static str = "support_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            support_channel_id: None,
            faq_channel_id: None,
            solved_tag_id: None,
            helper_role_id: None,
            solved_archive_secs: 60,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, support_channel_id, faq_channel_id, solved_tag_id,
                   helper_role_id, solved_archive_secs
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
            INSERT INTO support_settings (guild_id, support_channel_id, faq_channel_id,
                                          solved_tag_id, helper_role_id, solved_archive_secs)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (guild_id) DO UPDATE SET
                support_channel_id = EXCLUDED.support_channel_id,
                faq_channel_id = EXCLUDED.faq_channel_id,
                solved_tag_id = EXCLUDED.solved_tag_id,
                helper_role_id = EXCLUDED.helper_role_id,
                solved_archive_secs = EXCLUDED.solved_archive_secs,
                updated_at = now()
            RETURNING guild_id, support_channel_id, faq_channel_id, solved_tag_id,
                      helper_role_id, solved_archive_secs
            "#,
            self.guild_id,
            self.support_channel_id,
            self.faq_channel_id,
            self.solved_tag_id,
            self.helper_role_id,
            self.solved_archive_secs
        )
        .fetch_one(pool)
        .await
    }
}
