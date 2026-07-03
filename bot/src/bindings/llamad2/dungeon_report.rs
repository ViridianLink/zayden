use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

use super::llama_guild;

pub(super) struct DungeonReport;

#[async_trait]
impl ModuleCommand for DungeonReport {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("dungeonreport")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::DungeonReport::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Owned(llama_guild().into_iter().collect()))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::DungeonReport::run(cx.ctx, cx.interaction).await?;
        Ok(())
    }
}
