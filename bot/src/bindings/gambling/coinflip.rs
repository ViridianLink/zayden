use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};
use crate::BotState;

pub struct Coinflip;

#[async_trait]
impl ModuleCommand for Coinflip {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("coinflip")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_coinflip()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::coinflip::<
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
