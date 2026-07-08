use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use marathon::commands::Command as MarathonCommand;
use serenity::all::CreateCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Marathon;

#[async_trait]
impl ModuleCommand for Marathon {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("marathon")
    }

    fn definition(&self) -> CreateCommand<'static> {
        MarathonCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.marathon);
        drop(guard);

        MarathonCommand::run(cx, &client).await?;
        Ok(())
    }
}
