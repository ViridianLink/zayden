use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use destiny2::commands::Command as Destiny2Command;
use tokio::sync::RwLock;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

use crate::BotState;

pub struct Destiny2;

#[async_trait]
impl ModuleAutocomplete for Destiny2 {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("destiny2")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let Some(option) = cx.interaction.data.autocomplete() else {
            return Ok(());
        };

        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.bungie_client);
        drop(guard);

        Destiny2Command::autocomplete(
            cx.ctx,
            cx.interaction,
            option,
            &cx.app.db,
            &client,
            &cx.app.google_api_key,
        )
        .await?;

        Ok(())
    }
}
