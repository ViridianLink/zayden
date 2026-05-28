use async_trait::async_trait;
use serenity::all::{GuildId, MessageId};
use sqlx::{PgPool, Postgres};
use ticket::{
    TicketGuildManager,
    support_guild_manager::TicketGuildRow,
    ticket_manager::{TicketManager, TicketRow},
};

use zayden_app::config::ConfigStore;

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

        let Some(cfg) = ConfigStore::from_pool(pool.clone())
            .try_get(id.get() as i64)
            .await?
        else {
            return Ok(None);
        };

        // guild_support_roles is a separate table — still queried directly.
        let support_role_ids: Vec<i64> =
            sqlx::query_scalar("SELECT role_id FROM guild_support_roles WHERE guild_id = $1")
                .bind(id.get() as i64)
                .fetch_all(pool)
                .await?;

        Ok(Some(TicketGuildRow {
            id: cfg.id,
            thread_id: cfg.thread_id,
            support_channel_id: cfg.support_channel_id,
            support_role_ids,
            faq_channel_id: cfg.faq_channel_id,
        }))
    }

    async fn update_thread_id(pool: &PgPool, id: impl Into<GuildId> + Send) -> sqlx::Result<()> {
        ConfigStore::from_pool(pool.clone())
            .increment_thread_id(id.into().get() as i64)
            .await
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
        sqlx::query!(
            "DELETE FROM tickets WHERE thread_id = $1",
            id.into().get() as i64
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
