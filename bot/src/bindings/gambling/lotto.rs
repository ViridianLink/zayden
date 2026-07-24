use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Lotto;

#[async_trait]
impl ModuleCommand for Lotto {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("lotto")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_lotto()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::lotto::<BotState>(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
