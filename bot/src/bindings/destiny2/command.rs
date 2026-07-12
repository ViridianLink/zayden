use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use destiny2::commands::Command as Destiny2Command;
use serenity::all::CreateCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::{BotState, ZAYDEN_TOKEN, zayden_token};

pub struct Destiny2;

#[async_trait]
impl ModuleCommand for Destiny2 {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("destiny2")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Destiny2Command::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.bungie_client);
        drop(guard);

        let parent_token = ZAYDEN_TOKEN
            .get_or_try_init(|| zayden_token(&cx.app.db))
            .await
            .map_err(HandlerError::Database)?;

        Destiny2Command::run::<BotState>(
            cx.ctx,
            cx.interaction,
            &client,
            &cx.app.google_api_key,
            parent_token,
        )
        .await?;

        Ok(())
    }
}
