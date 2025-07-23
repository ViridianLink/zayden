use async_trait::async_trait;
use reaction_roles::ReactionRolesManager;
use reaction_roles::reaction_roles_manager::ReactionRole;
use serenity::all::{Context, CreateCommand, GenericChannelId, GuildId, MessageId, RoleId};
use sqlx::{PgPool, Postgres, postgres::PgQueryResult};
use zayden_core::SlashCommand;

pub use slash_command::ReactionRoleCommand;

pub mod slash_command;

pub fn register(ctx: &Context) -> CreateCommand<'_> {
    ReactionRoleCommand::register(ctx).unwrap()
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
            guild_id.get() as i64,
            channel_id.get() as i64,
            message_id.get() as i64,
            role_id.get() as i64,
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
            guild_id.get() as i64
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
            message_id.get() as i64,
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

        sqlx::query!("DELETE FROM reaction_roles WHERE guild_id = $1 AND channel_id = $2 AND message_id = $3 AND emoji = $4", guild_id.get() as i64, channel_id.get() as i64, message_id.get() as i64, emoji)
            .execute(pool)
            .await
    }
}
