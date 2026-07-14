use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use palworld::commands::Command as PalworldCommand;
use tokio::sync::RwLock;
use zayden_core::ctx::ModalCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleModal;
use zayden_core::scope::IdMatch;

use crate::BotState;

pub(super) struct PalworldUploadModal;

#[async_trait]
impl ModuleModal for PalworldUploadModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("palworld_save_upload"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let client = Arc::clone(&guard.palworld);
        drop(guard);

        PalworldCommand::upload_submit(cx, &client, &cx.app.db).await?;
        Ok(())
    }
}
