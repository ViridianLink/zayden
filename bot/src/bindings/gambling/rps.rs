use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct RockPaperScissors;

#[async_trait]
impl ModuleCommand for RockPaperScissors {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("rps")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_rps()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::rps::<BotState, Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
