use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

pub(super) struct Sensitivity;

#[async_trait]
impl ModuleCommand for Sensitivity {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("sensitivity")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::Sensitivity::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Borrowed(&super::LLAMA_GUILDS))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::Sensitivity::run(cx.ctx, cx.interaction)
            .await
            .map_err(HandlerError::new)?;
        Ok(())
    }
}
