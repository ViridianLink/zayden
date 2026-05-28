use std::borrow::Cow;

use async_trait::async_trait;
use suggestions::Suggestions;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

// --- Accept component (and its legacy aliases) ---

pub struct SuggestionsAccept;
pub struct SuggestionsAdded;
pub struct AcceptLegacy;

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

#[async_trait]
impl ModuleComponent for SuggestionsAdded {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_added"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::components(&cx.ctx.http, cx.interaction, true).await;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for AcceptLegacy {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("accept"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::components(&cx.ctx.http, cx.interaction, true).await;
        Ok(())
    }
}

// --- Reject component (and its legacy alias) ---

pub struct SuggestionsReject;
pub struct RejectLegacy;

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

#[async_trait]
impl ModuleComponent for RejectLegacy {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("reject"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::components(&cx.ctx.http, cx.interaction, false).await;
        Ok(())
    }
}

// --- Accept modal ---

pub struct SuggestionsAcceptModal;

#[async_trait]
impl ModuleModal for SuggestionsAcceptModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_accept"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::modal(&cx.ctx.http, cx.interaction, true)
            .await
            .map_err(HandlerError::from_respond)
    }
}

// --- Reject modal ---

pub struct SuggestionsRejectModal;

#[async_trait]
impl ModuleModal for SuggestionsRejectModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("suggestions_reject"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        Suggestions::modal(&cx.ctx.http, cx.interaction, false)
            .await
            .map_err(HandlerError::from_respond)
    }
}
