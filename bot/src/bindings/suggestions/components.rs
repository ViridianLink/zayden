use std::borrow::Cow;

use async_trait::async_trait;
use suggestions::Suggestions;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

pub(super) struct SuggestionsAccept;

#[async_trait]
impl ModuleComponent for SuggestionsAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::components(&cx.ctx.http, cx.interaction, true).await;
        Ok(())
    }
}

pub(super) struct SuggestionsReject;

#[async_trait]
impl ModuleComponent for SuggestionsReject {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_reject"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::components(&cx.ctx.http, cx.interaction, false).await;
        Ok(())
    }
}

pub(super) struct SuggestionsAcceptModal;

#[async_trait]
impl ModuleModal for SuggestionsAcceptModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_accept"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::modal(&cx.ctx.http, cx.interaction, true).await?;
        Ok(())
    }
}

pub(super) struct SuggestionsRejectModal;

#[async_trait]
impl ModuleModal for SuggestionsRejectModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_reject"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::modal(&cx.ctx.http, cx.interaction, false).await?;
        Ok(())
    }
}
