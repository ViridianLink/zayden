use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use crate::BotState;

pub struct Shop;

#[async_trait]
impl ModuleCommand for Shop {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("shop")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_shop()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::shop::<BotState>(cx.ctx, cx.interaction, options, &cx.app.db)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Shop {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("shop_"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        gambling::components::Shop::run_components::<BotState>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
