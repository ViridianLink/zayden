use serenity::all::{GenericChannelId, GuildId, RoleId};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
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

    pub async fn insert(
        pool: &PgPool,
        id: GuildId,
        channel: GenericChannelId,
        role: Option<RoleId>,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            r#"
            INSERT INTO lfg_settings (guild_id, lfg_channel_id, lfg_role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE SET
                lfg_channel_id = EXCLUDED.lfg_channel_id,
                lfg_role_id = EXCLUDED.lfg_role_id,
                updated_at = now()
            "#,
            as_i64(id.get()),
            as_i64(channel.get()),
            role.map(|r| as_i64(r.get()))
        )
        .execute(pool)
        .await
    }
}
