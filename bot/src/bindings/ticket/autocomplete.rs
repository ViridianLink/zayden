use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use ticket::Ticket;
use tokio::sync::RwLock;
use zayden_core::ctx::AutocompleteCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleAutocomplete;

use crate::BotState;

pub(crate) struct TicketAutocomplete;

#[async_trait]
impl ModuleAutocomplete for TicketAutocomplete {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("ticket")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let index = Arc::clone(&guard.wiki_index);
        drop(guard);

        Ticket::faq_autocomplete(&cx.ctx.http, cx.interaction, &cx.app, &index)
            .await?;

        Ok(())
    }
}
