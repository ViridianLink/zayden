use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

use super::llama_guild;

pub(super) struct Hello;

#[async_trait]
impl ModuleCommand for Hello {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("hello")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::Hello::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Owned(llama_guild().into_iter().collect()))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::Hello::run(cx.ctx, cx.interaction).await?;
        Ok(())
    }
}
