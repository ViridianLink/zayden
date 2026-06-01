use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};
use crate::BotState;

pub struct TicTacToe;

#[async_trait]
impl ModuleCommand for TicTacToe {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("tictactoe")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_tictactoe()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::tictactoe::<
            BotState,
            Postgres,
            GamblingTable,
            GoalsTable,
            EffectsTable,
            GameTable,
        >(cx.ctx, cx.interaction, options, &cx.app.db)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for TicTacToe {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("ttt"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        gambling::components::TicTacToe::run_component::<
            BotState,
            Postgres,
            GamblingTable,
            EffectsTable,
            GameTable,
        >(cx.ctx, cx.interaction, &cx.app.db)
        .await?;
        Ok(())
    }
}
