use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use crate::BotState;

pub struct Levels;

#[async_trait]
impl ModuleCommand for Levels {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("levels")
    }

    fn definition(&self) -> CreateCommand<'static> {
        levels::Levels::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        levels::Levels::run::<BotState>(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Levels {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("levels"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        levels::Levels::run_components::<BotState>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct Rank;

#[async_trait]
impl ModuleCommand for Rank {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("rank")
    }

    fn definition(&self) -> CreateCommand<'static> {
        levels::Rank::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        levels::Rank::rank(&cx.ctx.http, cx.interaction, options, &cx.app.db)
            .await?;
        Ok(())
    }
}

pub struct Xp;

#[async_trait]
impl ModuleCommand for Xp {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("xp")
    }

    fn definition(&self) -> CreateCommand<'static> {
        levels::Xp::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        levels::Xp::xp(&cx.ctx.http, cx.interaction, options, &cx.app.db).await?;
        Ok(())
    }
}
