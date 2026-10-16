use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use deadlock::autocomplete;
use tokio::sync::RwLock;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

use crate::BotState;

pub struct Deadlock;

#[async_trait]
impl ModuleAutocomplete for Deadlock {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("deadlock")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let api = Arc::clone(&guard.deadlock);
        drop(guard);

        autocomplete::run(cx, &api).await?;
        Ok(())
    }
}
