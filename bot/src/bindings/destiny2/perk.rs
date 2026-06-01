use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::error::HandlerError;
use zayden_core::{
    AutocompleteCtx,
    InvocationCtx,
    ModuleAutocomplete,
    ModuleCommand,
};

pub struct Perk;

#[async_trait]
impl ModuleCommand for Perk {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("perk")
    }

    fn definition(&self) -> CreateCommand<'static> {
        destiny2::slash_commands::perk::Perk::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        let _ = destiny2::slash_commands::perk::Perk::run(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.google_api_key,
        )
        .await;
        Ok(())
    }
}

#[async_trait]
impl ModuleAutocomplete for Perk {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("perk")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        if let Some(option) = cx.interaction.data.autocomplete() {
            destiny2::slash_commands::perk::Perk::autocomplete(
                &cx.ctx.http,
                cx.interaction,
                option,
                &cx.app.google_api_key,
            )
            .await
            .map_err(HandlerError::new)?;
        }
        Ok(())
    }
}
