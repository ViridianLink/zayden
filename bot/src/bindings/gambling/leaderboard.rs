use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use crate::BotState;

pub struct Leaderboard;

#[async_trait]
impl ModuleCommand for Leaderboard {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("leaderboard")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_leaderboard()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::leaderboard::<BotState>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Leaderboard {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("leaderboard"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        gambling::Leaderboard::run_component::<BotState>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
