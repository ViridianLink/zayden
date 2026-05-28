use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{CommandScope, HandlerError, InvocationCtx, ModuleCommand};

pub struct Playlist;

#[async_trait]
impl ModuleCommand for Playlist {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("playlist")
    }

    fn definition(&self) -> CreateCommand<'static> {
        llamad2::Playlist::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Borrowed(&super::LLAMA_GUILDS))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        llamad2::Playlist::run(cx.ctx, cx.interaction).await;
        Ok(())
    }
}
