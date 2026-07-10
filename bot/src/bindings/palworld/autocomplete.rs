use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use palworld::autocomplete;
use tokio::sync::RwLock;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

use crate::BotState;

pub struct Palworld;

#[async_trait]
impl ModuleAutocomplete for Palworld {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("palworld")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.palworld);
        drop(guard);

        autocomplete::run(cx, &client).await?;
        Ok(())
    }
}
