use async_trait::async_trait;
use gambling::Commands;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct TicTacToe;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for TicTacToe {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::tictactoe::<CtxData, Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_tictactoe())
    }
}
