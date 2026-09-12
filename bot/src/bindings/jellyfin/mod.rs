pub mod command;
pub mod watch;

use std::borrow::Cow;
use std::sync::Arc;

use ::watch::components::{
    PARTY_CANCEL_PREFIX,
    PARTY_JOIN_PREFIX,
    PARTY_LEAVE_PREFIX,
    party,
};
use async_trait::async_trait;
pub use command::Jellyfin;
use jellyfin::JellyfinError;
use jellyfin::components::{
    GUESS_MODAL_PREFIX,
    GUESS_OPEN_PREFIX,
    TRIVIA_ANSWER_PREFIX,
    game,
    link_cancel,
};
use jellyfin::quick_connect::CANCEL_PREFIX;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::Context;
use tokio::sync::RwLock;
pub use watch::Watch;
use zayden_core::ctx::{ComponentCtx, ModalCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleComponent, ModuleModal};
use zayden_core::scope::IdMatch;

use crate::registry::OverlapError;
use crate::{BotState, RegistryBuilder};

pub(crate) async fn runtime(
    ctx: &Context,
) -> Result<Arc<JellyfinRuntime>, HandlerError> {
    let data = ctx.data::<RwLock<BotState>>();
    let guard = data.read().await;
    let found = guard.jellyfin.as_ref().map(Arc::clone);
    drop(guard);

    found.ok_or_else(|| HandlerError::from_respond(JellyfinError::NotConfigured))
}

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Jellyfin)
        .add_command(Watch)
        .add_autocomplete(Jellyfin)
        .add_autocomplete(Watch)
        .add_component(LinkCancel)?
        .add_component(PartyJoin)?
        .add_component(PartyLeave)?
        .add_component(PartyCancel)?
        .add_component(TriviaAnswer)?
        .add_component(GuessOpen)?
        .add_modal(GuessSubmit)?;

    Ok(())
}

fn suffix(cx: &ComponentCtx<'_>, prefix: &str) -> String {
    cx.interaction.data.custom_id.strip_prefix(prefix).unwrap_or_default().to_owned()
}

pub struct LinkCancel;

#[async_trait]
impl ModuleComponent for LinkCancel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(CANCEL_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        let suffix = suffix(cx, CANCEL_PREFIX);

        link_cancel::run(cx, &runtime, &suffix).await?;
        Ok(())
    }
}

pub struct PartyJoin;

#[async_trait]
impl ModuleComponent for PartyJoin {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(PARTY_JOIN_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        let suffix = suffix(cx, PARTY_JOIN_PREFIX);

        party::join(cx, &runtime, &suffix).await?;
        Ok(())
    }
}

pub struct PartyLeave;

#[async_trait]
impl ModuleComponent for PartyLeave {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(PARTY_LEAVE_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        let suffix = suffix(cx, PARTY_LEAVE_PREFIX);

        party::leave(cx, &runtime, &suffix).await?;
        Ok(())
    }
}

pub struct PartyCancel;

#[async_trait]
impl ModuleComponent for PartyCancel {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(PARTY_CANCEL_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        let suffix = suffix(cx, PARTY_CANCEL_PREFIX);

        let scheduled = party::cancel(cx, &runtime, &suffix).await?;
        watch::apply(cx.ctx, &runtime, scheduled).await;

        Ok(())
    }
}

pub struct TriviaAnswer;

#[async_trait]
impl ModuleComponent for TriviaAnswer {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(TRIVIA_ANSWER_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let suffix = cx
            .interaction
            .data
            .custom_id
            .strip_prefix(TRIVIA_ANSWER_PREFIX)
            .unwrap_or_default()
            .to_owned();

        game::trivia_answer(cx, &suffix).await?;
        Ok(())
    }
}

pub struct GuessOpen;

#[async_trait]
impl ModuleComponent for GuessOpen {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(GUESS_OPEN_PREFIX))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let suffix = cx
            .interaction
            .data
            .custom_id
            .strip_prefix(GUESS_OPEN_PREFIX)
            .unwrap_or_default()
            .to_owned();

        game::open_guess_modal(cx, &suffix).await?;
        Ok(())
    }
}

pub struct GuessSubmit;

#[async_trait]
impl ModuleModal for GuessSubmit {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed(GUESS_MODAL_PREFIX))
    }

    async fn run(&self, cx: &ModalCtx<'_>) -> Result<(), HandlerError> {
        let suffix = cx
            .interaction
            .data
            .custom_id
            .strip_prefix(GUESS_MODAL_PREFIX)
            .unwrap_or_default()
            .to_owned();

        game::guess_submit(cx, &suffix).await?;
        Ok(())
    }
}
