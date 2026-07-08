use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use marathon::autocomplete;
use tokio::sync::RwLock;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

use crate::BotState;

pub struct Marathon;

#[async_trait]
impl ModuleAutocomplete for Marathon {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("marathon")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.marathon);
        drop(guard);

        autocomplete::run(cx, &client).await?;
        Ok(())
    }
}
