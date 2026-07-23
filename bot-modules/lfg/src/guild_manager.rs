use async_trait::async_trait;
use serenity::all::{GenericChannelId, GuildId, RoleId};
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;

use crate::commands::SetupManager;
use crate::modals::create::{GuildManager, GuildRow};

pub struct GuildTable;

#[async_trait]
impl GuildManager<Postgres> for GuildTable {
    async fn row(pool: &PgPool, id: GuildId) -> sqlx::Result<Option<GuildRow>> {
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

#[async_trait]
impl SetupManager<Postgres> for GuildTable {
    async fn insert(
        pool: &PgPool,
        id: GuildId,
        channel: GenericChannelId,
        role: Option<RoleId>,
    ) -> sqlx::Result<sqlx::postgres::PgQueryResult> {
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
