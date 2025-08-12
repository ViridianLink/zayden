use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use ticket::{Support, Ticket};
use zayden_core::ApplicationCommand;

use crate::sqlx_lib::GuildTable;
use crate::{Error, Result};

use super::TicketTable;

pub struct TicketCommand;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for TicketCommand {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Ticket::run::<Postgres, GuildTable, TicketTable>(&ctx.http, interaction, pool, options)
            .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Ticket::register())
    }
}

pub struct SupportCommand;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for SupportCommand {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Support::run::<Postgres, GuildTable>(&ctx.http, interaction, pool, options).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Support::register())
    }
}
