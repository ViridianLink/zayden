use std::borrow::Cow;

use ::hosting::commands::Hosting;
use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::ctx::{AutocompleteCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleAutocomplete, ModuleCommand};

use super::runtime;

async fn dispatch(cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
    let runtime = runtime(cx.ctx).await?;

    Hosting::run(
        &cx.ctx.http,
        cx.interaction,
        &runtime,
        &cx.app.db,
        &cx.app.entitlements,
        cx.interaction.data.options(),
    )
    .await?;

    Ok(())
}

async fn complete(cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
    let runtime = runtime(cx.ctx).await?;

    Hosting::autocomplete(&cx.ctx.http, cx.interaction, &runtime).await?;
    Ok(())
}

pub struct Server;

#[async_trait]
impl ModuleCommand for Server {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("server")
    }

    fn module(&self) -> Option<&'static str> {
        Some("hosting")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Hosting::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        // Delegating to a plain `async fn`: inside `async_trait` the compiler
        // cannot prove the higher-ranked lifetime bound for a dispatch this
        // wide (rust-lang/rust#100013).
        dispatch(cx).await
    }
}

#[async_trait]
impl ModuleAutocomplete for Server {
    fn command(&self) -> Cow<'static, str> {
        Cow::Borrowed("server")
    }

    async fn run(&self, cx: &AutocompleteCtx<'_>) -> Result<(), HandlerError> {
        complete(cx).await
    }
}
