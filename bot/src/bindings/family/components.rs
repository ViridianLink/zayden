use std::borrow::Cow;

use async_trait::async_trait;
use family::components;
use serenity::all::{CreateInteractionResponse, CreateInteractionResponseMessage};
use zayden_core::ctx::ComponentCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleComponent;
use zayden_core::scope::IdMatch;

async fn update(
    cx: &ComponentCtx<'_>,
    content: &'static str,
) -> Result<(), HandlerError> {
    cx.interaction
        .create_response(
            &cx.ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .components(vec![]),
            ),
        )
        .await?;

    Ok(())
}

pub(super) struct MarryAccept;

#[async_trait]
impl ModuleComponent for MarryAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::MARRY_ACCEPT))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        components::marry::accept(cx.interaction, &cx.app.db).await?;

        update(cx, "Congratulations! You are now married!").await
    }
}

pub(super) struct MarryDecline;

#[async_trait]
impl ModuleComponent for MarryDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::MARRY_DECLINE))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        components::marry::decline(cx.interaction)?;

        update(cx, "Marriage proposal declined.").await
    }
}

pub(super) struct AdoptAccept;

#[async_trait]
impl ModuleComponent for AdoptAccept {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::ADOPT_ACCEPT))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        components::adopt::accept(cx.interaction, &cx.app.db).await?;

        update(cx, "Adoption accepted! Welcome to the family!").await
    }
}

pub(super) struct AdoptDecline;

#[async_trait]
impl ModuleComponent for AdoptDecline {
    fn id_match(&self) -> IdMatch {
        IdMatch::Exact(Cow::Borrowed(components::ADOPT_DECLINE))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        components::adopt::decline(cx.interaction)?;

        update(cx, "Adoption declined.").await
    }
}
