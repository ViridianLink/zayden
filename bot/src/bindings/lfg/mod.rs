mod slash_command;

use std::borrow::Cow;

use async_trait::async_trait;
use lfg::{Components, KickComponent, TagsComponent};
pub use slash_command::Lfg;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

use crate::registry::OverlapError;
use crate::{BotState, RegistryBuilder};

// region: Components

pub struct LfgJoin;

#[async_trait]
impl ModuleComponent for LfgJoin {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_join"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::join(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgLeave;

#[async_trait]
impl ModuleComponent for LfgLeave {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_leave"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::leave(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgAlternative;

#[async_trait]
impl ModuleComponent for LfgAlternative {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_alternative"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::alternative(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgSettings;

#[async_trait]
impl ModuleComponent for LfgSettings {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_settings"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::settings(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgEditComponent;

#[async_trait]
impl ModuleComponent for LfgEditComponent {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_edit"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::edit(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgCopy;

#[async_trait]
impl ModuleComponent for LfgCopy {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_copy"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::copy(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgKick;

#[async_trait]
impl ModuleComponent for LfgKick {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_kick"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::kick(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgKickMenu;

#[async_trait]
impl ModuleComponent for LfgKickMenu {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_kick_menu"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        KickComponent::run(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgDelete;

#[async_trait]
impl ModuleComponent for LfgDelete {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_delete"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        Components::delete(&cx.ctx.http, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

pub struct LfgTagsAdd;

#[async_trait]
impl ModuleComponent for LfgTagsAdd {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_tags_add"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TagsComponent::add(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

pub struct LfgTagsRemove;

#[async_trait]
impl ModuleComponent for LfgTagsRemove {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_tags_remove"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        TagsComponent::remove(&cx.ctx.http, cx.interaction).await?;
        Ok(())
    }
}

// endregion

// region: Modals

pub struct LfgEditModal;

#[async_trait]
impl ModuleModal for LfgEditModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed("lfg_edit"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        lfg::modals::Edit::run::<BotState>(cx.ctx, cx.interaction, &cx.app.db)
            .await?;
        Ok(())
    }
}

pub struct LfgCreateModal;

#[async_trait]
impl ModuleModal for LfgCreateModal {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("lfg_create"))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        lfg::modals::Create::run::<BotState>(cx.ctx, cx.interaction, &cx.app.db)
            .await?;
        Ok(())
    }
}

// endregion

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Lfg)
        .add_autocomplete(Lfg)
        .add_component(LfgJoin)?
        .add_component(LfgLeave)?
        .add_component(LfgAlternative)?
        .add_component(LfgSettings)?
        .add_component(LfgEditComponent)?
        .add_component(LfgCopy)?
        .add_component(LfgKick)?
        .add_component(LfgKickMenu)?
        .add_component(LfgDelete)?
        .add_component(LfgTagsAdd)?
        .add_component(LfgTagsRemove)?
        .add_modal(LfgEditModal)?
        .add_modal(LfgCreateModal)?;

    Ok(())
}
