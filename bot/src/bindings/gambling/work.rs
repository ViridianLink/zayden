use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Work;

#[async_trait]
impl ModuleCommand for Work {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("work")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_work()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::work::<BotState>(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
