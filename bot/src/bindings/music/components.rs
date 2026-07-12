use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use music::components::{
    CONTROL_PANEL_PREFIX,
    ControlPanel as MusicControlPanel,
    PanelCtx,
    QUEUE_PAGER_PREFIX,
    QueuePager as MusicQueuePager,
};
use serenity::all::UserId;
use tokio::sync::RwLock;
use zayden_core::ctx::ComponentCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleComponent;
use zayden_core::scope::IdMatch;

use crate::BotState;

pub struct ControlPanel;

#[async_trait]
impl ModuleComponent for ControlPanel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(CONTROL_PANEL_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let songbird = Arc::clone(&guard.songbird);
        let music = Arc::clone(&guard.music);
        let resolver = Arc::clone(&guard.music_resolver);
        drop(guard);

        let settings = Arc::clone(&cx.app.settings.music);

        let ctx = PanelCtx::new(
            &cx.ctx.http,
            cx.interaction,
            UserId::new(cx.app.zayden_id),
            songbird,
            music,
            resolver,
            settings,
        )?;

        let action = cx
            .interaction
            .data
            .custom_id
            .strip_prefix(CONTROL_PANEL_PREFIX)
            .unwrap_or_default();

        MusicControlPanel::run(&ctx, action).await?;
        Ok(())
    }
}

pub struct QueuePager;

#[async_trait]
impl ModuleComponent for QueuePager {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(QUEUE_PAGER_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let songbird = Arc::clone(&guard.songbird);
        let music = Arc::clone(&guard.music);
        let resolver = Arc::clone(&guard.music_resolver);
        drop(guard);

        let settings = Arc::clone(&cx.app.settings.music);

        let ctx = PanelCtx::new(
            &cx.ctx.http,
            cx.interaction,
            UserId::new(cx.app.zayden_id),
            songbird,
            music,
            resolver,
            settings,
        )?;

        let page_suffix = cx
            .interaction
            .data
            .custom_id
            .strip_prefix(QUEUE_PAGER_PREFIX)
            .unwrap_or_default();

        MusicQueuePager::run(&ctx, page_suffix).await?;
        Ok(())
    }
}
