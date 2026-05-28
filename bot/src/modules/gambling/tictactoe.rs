use async_trait::async_trait;
use gambling::Commands;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Component};

use crate::{BotState, Error, Result};

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct TicTacToe;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for TicTacToe {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::tictactoe::<BotState, Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_tictactoe()
    }
}

#[async_trait]
impl Component<Error, Postgres> for TicTacToe {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
        gambling::components::TicTacToe::run_component::<
            BotState,
            Postgres,
            GamblingTable,
            EffectsTable,
            GameTable,
        >(ctx, interaction, pool)
        .await?;

        Ok(())
    }
}
