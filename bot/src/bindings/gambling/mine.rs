use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Mine;

#[async_trait]
impl ModuleCommand for Mine {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("mine")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_mine()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::mine::<BotState>(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
