use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

use super::llama_guild;

pub(super) struct Goof;

#[async_trait]
impl ModuleCommand for Goof {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("goof")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::Goof::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Owned(llama_guild().into_iter().collect()))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::Goof::run(cx.ctx, cx.interaction).await?;
        Ok(())
    }
}
