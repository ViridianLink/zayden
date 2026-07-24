use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Send;

#[async_trait]
impl ModuleCommand for Send {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("send")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_send()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::send::<BotState>(cx.ctx, cx.interaction, options, &cx.app.db)
            .await?;
        Ok(())
    }
}
