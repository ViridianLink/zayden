use std::borrow::Cow;

use ::hosting::components;
use ::hosting::modal::{CANCEL_PREFIX, CONFIRM_PREFIX, MODAL_PREFIX};
use async_trait::async_trait;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

use super::runtime;

async fn submit(cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
    let runtime = runtime(cx.ctx).await?;

    components::spec_submitted(
        &cx.ctx.http,
        cx.interaction,
        &runtime,
        &cx.app.db,
        &cx.app.entitlements,
    )
    .await?;

    Ok(())
}

async fn confirm(cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
    let runtime = runtime(cx.ctx).await?;

    components::confirm(&cx.ctx.http, cx.interaction, &runtime, &cx.app.db).await?;

    Ok(())
}

pub struct HostingSpec;

#[async_trait]
impl ModuleModal for HostingSpec {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(MODAL_PREFIX))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        submit(cx).await
    }
}

pub struct HostingConfirm;

#[async_trait]
impl ModuleComponent for HostingConfirm {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(CONFIRM_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        confirm(cx).await
    }
}

pub struct HostingCancel;

#[async_trait]
impl ModuleComponent for HostingCancel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(CANCEL_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        components::cancel(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}
