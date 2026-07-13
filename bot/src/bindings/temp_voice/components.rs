use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use serenity::all::Context;
use sqlx::Postgres;
use temp_voice::VoiceStateCache;
use temp_voice::components::{self, Components};
use tokio::sync::RwLock;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

use super::VoiceChannelTable;
use crate::BotState;

async fn voice_states(ctx: &Context) -> Arc<VoiceStateCache> {
    Arc::clone(&ctx.data::<RwLock<BotState>>().read().await.voice_states)
}

pub(super) struct VoiceClaim;

#[async_trait]
impl ModuleComponent for VoiceClaim {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::CLAIM))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::claim::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceDelete;

#[async_trait]
impl ModuleComponent for VoiceDelete {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::DELETE))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::delete::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceRename;

#[async_trait]
impl ModuleComponent for VoiceRename {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::RENAME))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::rename::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceLimit;

#[async_trait]
impl ModuleComponent for VoiceLimit {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::LIMIT))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::limit::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceBitrate;

#[async_trait]
impl ModuleComponent for VoiceBitrate {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::BITRATE))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::bitrate::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoicePassword;

#[async_trait]
impl ModuleComponent for VoicePassword {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::PASSWORD))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::password::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoicePrivacy;

#[async_trait]
impl ModuleComponent for VoicePrivacy {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::PRIVACY))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::privacy::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceRegion;

#[async_trait]
impl ModuleComponent for VoiceRegion {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::REGION))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::region::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceTransfer;

#[async_trait]
impl ModuleComponent for VoiceTransfer {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::TRANSFER))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::transfer::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceTrust;

#[async_trait]
impl ModuleComponent for VoiceTrust {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::TRUST))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::trust::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceKick;

#[async_trait]
impl ModuleComponent for VoiceKick {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::KICK))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::kick::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoicePrivacyMenu;

#[async_trait]
impl ModuleComponent for VoicePrivacyMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::PRIVACY_MENU))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::privacy_menu::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceRegionMenu;

#[async_trait]
impl ModuleComponent for VoiceRegionMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::REGION_MENU))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::region_menu::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceTransferMenu;

#[async_trait]
impl ModuleComponent for VoiceTransferMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::TRANSFER_MENU))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::transfer_menu::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceTrustMenu;

#[async_trait]
impl ModuleComponent for VoiceTrustMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::TRUST_MENU))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::trust_menu::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceKickMenu;

#[async_trait]
impl ModuleComponent for VoiceKickMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::KICK_MENU))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::kick_menu::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceRenameModal;

#[async_trait]
impl ModuleModal for VoiceRenameModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::RENAME))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::rename_submit::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceLimitModal;

#[async_trait]
impl ModuleModal for VoiceLimitModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::LIMIT))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::limit_submit::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoiceBitrateModal;

#[async_trait]
impl ModuleModal for VoiceBitrateModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::BITRATE))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::bitrate_submit::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}

pub(super) struct VoicePasswordModal;

#[async_trait]
impl ModuleModal for VoicePasswordModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::PASSWORD))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let voice_states = voice_states(cx.ctx).await;

        Components::password_submit::<Postgres, VoiceChannelTable>(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;

        Ok(())
    }
}
