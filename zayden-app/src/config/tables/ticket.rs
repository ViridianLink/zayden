use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TicketSettingsRow {
    pub guild_id: i64,
    pub thread_id: i32,
}

impl SettingsRow for TicketSettingsRow {
    const TABLE: &'static str = "ticket_settings";

    fn empty(guild_id: i64) -> Self {
        Self { guild_id, thread_id: 0 }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, thread_id
            FROM ticket_settings
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
            INSERT INTO ticket_settings (guild_id, thread_id)
            VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE SET
                thread_id = EXCLUDED.thread_id,
                updated_at = now()
            RETURNING guild_id, thread_id
            "#,
            self.guild_id,
            self.thread_id
        )
        .fetch_one(pool)
        .await
    }
}
