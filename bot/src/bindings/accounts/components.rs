use std::borrow::Cow;

use accounts::commands::{UNLINK_CANCEL_ID, UNLINK_CONFIRM_PREFIX, UnlinkCommand};
use async_trait::async_trait;
use zayden_core::ctx::ComponentCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleComponent;
use zayden_core::scope::IdMatch;

pub(super) struct UnlinkConfirm;

#[async_trait]
impl ModuleComponent for UnlinkConfirm {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(UNLINK_CONFIRM_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        UnlinkCommand::confirm(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub(super) struct UnlinkCancel;

#[async_trait]
impl ModuleComponent for UnlinkCancel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(UNLINK_CANCEL_ID))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        UnlinkCommand::cancel(cx.ctx, cx.interaction).await?;
        Ok(())
    }
}
