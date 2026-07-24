use serenity::all::GuildId;
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::modals::create::GuildRow;

impl GuildRow {
    pub async fn get(pool: &PgPool, id: GuildId) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            GuildRow,
            r#"
            SELECT lfg_channel_id, lfg_role_id, lfg_scheduled_thread_id
            FROM lfg_settings
            WHERE guild_id = $1
            "#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }
}
