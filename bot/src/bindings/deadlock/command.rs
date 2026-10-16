use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use deadlock::commands::Command as DeadlockCommand;
use serenity::all::CreateCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Deadlock;

#[async_trait]
impl ModuleCommand for Deadlock {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("deadlock")
    }

    fn module(&self) -> Option<&'static str> {
        Some("deadlock")
    }

    fn definition(&self) -> CreateCommand<'static> {
        DeadlockCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let api = Arc::clone(&guard.deadlock);
        drop(guard);

        DeadlockCommand::run(cx, &api).await?;
        Ok(())
    }
}
