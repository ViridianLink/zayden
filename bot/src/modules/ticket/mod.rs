use async_trait::async_trait;
use serenity::all::{GuildId, MessageId};
use sqlx::{PgPool, Postgres};
use ticket::{
    TicketGuildManager,
    support_guild_manager::TicketGuildRow,
    ticket_manager::{TicketManager, TicketRow},
};

use crate::sqlx_lib::GuildTable;

pub mod components;
pub mod message_commands;
pub mod slash_commands;

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
               r#"
                SELECT 
                    gc.id, 
                    gc.thread_id, 
                    gc.support_channel_id, 
                    COALESCE(
                        (SELECT array_agg(role_id) FROM guild_support_roles WHERE guild_id = gc.id), 
                        ARRAY[]::bigint[]
                    ) AS "support_role_ids!", 
                    gc.faq_channel_id 
                FROM guild_config gc 
                WHERE gc.id = $1;
                "#,
                id.get() as i64
            )
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    async fn update_thread_id(pool: &PgPool, id: impl Into<GuildId> + Send) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE guild_config SET thread_id = thread_id + 1 WHERE id = $1",
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
            r#"SELECT thread_id, COALESCE(
                        (SELECT array_agg(role_id) FROM ticket_roles WHERE ticket_id = t.thread_id), 
                        ARRAY[]::bigint[]
                    ) AS "role_ids!" FROM tickets t WHERE thread_id = $1"#,
            id.into().get() as i64
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    async fn delete(pool: &PgPool, id: impl Into<MessageId> + Send) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM tickets WHERE thread_id = $1", id.into().get() as i64)
            .execute(pool)
            .await?;

        Ok(())
    }
}
