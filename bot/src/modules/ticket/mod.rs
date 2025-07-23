use async_trait::async_trait;
use serenity::all::{Context, CreateCommand, GuildId, MessageId};
use slash_commands::{SupportCommand, TicketCommand};
use sqlx::{PgPool, Postgres};
use ticket::{
    TicketGuildManager,
    support_guild_manager::TicketGuildRow,
    ticket_manager::{TicketManager, TicketRow},
};
use zayden_core::SlashCommand;

use crate::sqlx_lib::GuildTable;

pub mod components;
pub mod message_commands;
pub mod slash_commands;

pub fn register(ctx: &Context) -> [CreateCommand<'_>; 2] {
    [
        TicketCommand::register(ctx).unwrap(),
        SupportCommand::register(ctx).unwrap(),
    ]
}

pub struct Ticket;

#[async_trait]
impl TicketGuildManager<Postgres> for GuildTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<TicketGuildRow>> {
        let id = id.into();

        let row = sqlx::query_as!(
                TicketGuildRow,
                r#"SELECT id, thread_id, support_channel_id, support_role_ids, faq_channel_id FROM guilds WHERE id = $1"#,
                id.get() as i64
            )
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    async fn update_thread_id(pool: &PgPool, id: impl Into<GuildId> + Send) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE guilds SET thread_id = thread_id + 1 WHERE id = $1",
            id.into().get() as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

pub struct TicketTable;

#[async_trait]
impl TicketManager<Postgres> for TicketTable {
    async fn get(pool: &PgPool, id: impl Into<MessageId> + Send) -> sqlx::Result<TicketRow> {
        let row = sqlx::query_as!(
            TicketRow,
            "SELECT * FROM tickets WHERE id = $1",
            id.into().get() as i64
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    async fn delete(pool: &PgPool, id: impl Into<MessageId> + Send) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM tickets WHERE id = $1", id.into().get() as i64)
            .execute(pool)
            .await?;

        Ok(())
    }
}
