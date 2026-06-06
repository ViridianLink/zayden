use async_trait::async_trait;
use reaction_roles::ReactionRolesManager;
use reaction_roles::reaction_roles_manager::ReactionRole;
use serenity::all::{GenericChannelId, GuildId, MessageId, RoleId};
pub use slash_command::ReactionRoleCommand;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::as_i64;

pub mod slash_command;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(ReactionRoleCommand);
}

pub struct ReactionRolesTable;

#[async_trait]
impl ReactionRolesManager<Postgres> for ReactionRolesTable {
    async fn create(
        pool: &PgPool,
        guild_id: impl Into<GuildId> + Send,
        channel_id: impl Into<GenericChannelId> + Send,
        message_id: impl Into<MessageId> + Send,
        role_id: impl Into<RoleId> + Send,
        emoji: &str,
    ) -> sqlx::Result<PgQueryResult> {
        let guild_id = guild_id.into();
        let channel_id = channel_id.into();
        let message_id = message_id.into();
        let role_id = role_id.into();

        sqlx::query_file!(
            "sql/reaction_roles/create.sql",
            as_i64(guild_id.get()),
            as_i64(channel_id.get()),
            as_i64(message_id.get()),
            as_i64(role_id.get()),
            emoji
        )
        .execute(pool)
        .await
    }

    async fn rows(
        pool: &PgPool,
        guild_id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Vec<ReactionRole>> {
        let guild_id = guild_id.into();

        sqlx::query_as!(
            ReactionRole,
            "SELECT * FROM reaction_roles WHERE guild_id = $1",
            as_i64(guild_id.get())
        )
        .fetch_all(pool)
        .await
    }

    async fn row(
        pool: &PgPool,
        message_id: impl Into<MessageId> + Send,
        emoji: &str,
    ) -> sqlx::Result<Option<ReactionRole>> {
        let message_id = message_id.into();

        sqlx::query_as!(
            ReactionRole,
            "SELECT * FROM reaction_roles WHERE message_id = $1 AND emoji = $2",
            as_i64(message_id.get()),
            emoji
        )
        .fetch_optional(pool)
        .await
    }

    async fn delete(
        pool: &PgPool,
        guild_id: impl Into<GuildId> + Send,
        channel_id: impl Into<GenericChannelId> + Send,
        message_id: impl Into<MessageId> + Send,
        emoji: &str,
    ) -> sqlx::Result<PgQueryResult> {
        let guild_id = guild_id.into();
        let channel_id = channel_id.into();
        let message_id = message_id.into();

        sqlx::query!("DELETE FROM reaction_roles WHERE guild_id = $1 AND channel_id = $2 AND message_id = $3 AND emoji = $4", as_i64(guild_id.get()), as_i64(channel_id.get()), as_i64(message_id.get()), emoji)
            .execute(pool)
            .await
    }
}
