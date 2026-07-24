use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Daily;

#[async_trait]
impl ModuleCommand for Daily {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("daily")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_daily()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::daily::<BotState>(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
