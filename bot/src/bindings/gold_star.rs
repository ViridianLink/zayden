use std::borrow::Cow;

use async_trait::async_trait;
use gold_star::{GiveStar, Stars};
use serenity::all::CreateCommand;
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(GiveStarCmd);
    builder.add_command(StarsCmd);
}

pub struct GiveStarCmd;

#[async_trait]
impl ModuleCommand for GiveStarCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("give_star")
    }

    fn definition(&self) -> CreateCommand<'static> {
        GiveStar::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        GiveStar::run(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

pub struct StarsCmd;

#[async_trait]
impl ModuleCommand for StarsCmd {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("stars")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Stars::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Stars::run(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
