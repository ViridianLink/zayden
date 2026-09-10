use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::ctx::{AutocompleteCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleAutocomplete, ModuleCommand};

use super::runtime;
use crate::BotState;

async fn dispatch(cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
    let runtime = runtime(cx.ctx).await?;

    // The module cannot name `BotState`, so it hands back the cron work it
    // wants registered and this layer applies it.
    let scheduled = ::watch::Watch::run(cx, &runtime).await?;
    apply(cx.ctx, &runtime, scheduled).await;

    Ok(())
}

pub(super) async fn apply(
    ctx: &serenity::all::Context,
    runtime: &std::sync::Arc<::jellyfin::runtime::JellyfinRuntime>,
    scheduled: ::watch::party::Scheduled,
) {
    scheduled.apply::<BotState>(ctx, runtime).await;
}

pub struct Watch;

#[async_trait]
impl ModuleCommand for Watch {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("watch")
    }

    fn module(&self) -> Option<&'static str> {
        Some("jellyfin")
    }

    fn definition(&self) -> CreateCommand<'static> {
        ::watch::Watch::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        // Delegating to a plain `async fn` rather than writing the body inline:
        // inside `async_trait` the compiler cannot prove the higher-ranked
        // lifetime bound for a dispatch this wide (rust-lang/rust#100013).
        dispatch(cx).await
    }
}

#[async_trait]
impl ModuleAutocomplete for Watch {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("watch")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        ::watch::autocomplete::run(cx, &runtime).await?;
        Ok(())
    }
}
