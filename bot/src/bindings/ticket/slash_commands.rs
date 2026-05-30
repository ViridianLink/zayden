use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use ticket::{Support, Ticket};
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use super::TicketTable;
use crate::sqlx_lib::GuildTable;

pub struct TicketCommand;

#[async_trait]
impl ModuleCommand for TicketCommand {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("ticket")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Ticket::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Ticket::run::<Postgres, GuildTable, TicketTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            cx.interaction.data.options(),
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}

pub struct SupportCommand;

#[async_trait]
impl ModuleCommand for SupportCommand {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("support")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Support::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Support::run::<Postgres, GuildTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            cx.interaction.data.options(),
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
