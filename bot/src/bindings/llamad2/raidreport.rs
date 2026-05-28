use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

pub struct RaidReport;

#[async_trait]
impl ModuleCommand for RaidReport {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("raidreport")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::RaidReport::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Borrowed(&super::LLAMA_GUILDS))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::RaidReport::run(cx.ctx, cx.interaction).await;
        Ok(())
    }
}
