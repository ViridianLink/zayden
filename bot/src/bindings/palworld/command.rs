use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use palworld::commands::Command as PalworldCommand;
use serenity::all::CreateCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Palworld;

#[async_trait]
impl ModuleCommand for Palworld {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("palworld")
    }

    fn definition(&self) -> CreateCommand<'static> {
        PalworldCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.palworld);
        drop(guard);

        PalworldCommand::run(cx, &client).await?;
        Ok(())
    }
}
