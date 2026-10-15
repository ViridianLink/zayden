use std::borrow::Cow;
use std::sync::Arc;

use accounts::commands::{LinkCommand, UnlinkCommand};
use async_trait::async_trait;
use serenity::all::CreateCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Link;

#[async_trait]
impl ModuleCommand for Link {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("link")
    }

    fn definition(&self) -> CreateCommand<'static> {
        LinkCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let dashboard_url = Arc::clone(&guard.dashboard_url);
        drop(guard);

        LinkCommand::run(cx.ctx, cx.interaction, &cx.app.db, &dashboard_url).await?;
        Ok(())
    }
}

pub struct Unlink;

#[async_trait]
impl ModuleCommand for Unlink {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("unlink")
    }

    fn definition(&self) -> CreateCommand<'static> {
        UnlinkCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        UnlinkCommand::run(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
