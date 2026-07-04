use async_trait::async_trait;
use serenity::all::MessageId;
use sqlx::{PgPool, Postgres};
use ticket::ticket_manager::{TicketManager, TicketRow};
use zayden_core::as_i64;

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub mod components;
pub mod message_commands;
pub mod slash_commands;

use components::{
    CreateTicketModal,
    SupportClose,
    SupportFaq,
    SupportTicket,
    TicketCreate,
};
use slash_commands::{SupportCommand, TicketCommand};

pub struct TicketTable;

#[async_trait]
impl TicketManager<Postgres> for TicketTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<TicketRow> {
        let row = sqlx::query_as!(
            TicketRow,
            r#"SELECT thread_id, COALESCE(
                        (SELECT array_agg(role_id) FROM ticket_roles WHERE ticket_id = t.thread_id), 
                        ARRAY[]::bigint[]
                    ) AS "role_ids!" FROM tickets t WHERE thread_id = $1"#,
            as_i64(id.into().get())
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    async fn delete(
        pool: &PgPool,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "DELETE FROM tickets WHERE thread_id = $1",
            as_i64(id.into().get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(TicketCommand)
        .add_command(SupportCommand)
        .add_component(TicketCreate)?
        .add_component(SupportTicket)?
        .add_component(SupportClose)?
        .add_component(SupportFaq)?
        .add_modal(CreateTicketModal)?;

    Ok(())
}
