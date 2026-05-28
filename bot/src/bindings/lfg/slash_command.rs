use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use zayden_core::ctx::{AutocompleteCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleAutocomplete, ModuleCommand};

use super::{PostTable, UsersTable};

pub struct Lfg;

#[async_trait]
impl ModuleCommand for Lfg {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("lfg")
    }

    fn definition(&self) -> CreateCommand<'static> {
        lfg::Command::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        lfg::Command::lfg::<Postgres, UsersTable, PostTable>(
            &cx.ctx.http,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}

#[async_trait]
impl ModuleAutocomplete for Lfg {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("lfg")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let Some(option) = cx.interaction.data.autocomplete() else {
            return Ok(());
        };
        lfg::Command::autocomplete(&cx.ctx.http, cx.interaction, option)
            .await
            .map_err(HandlerError::from_respond)
    }
}
