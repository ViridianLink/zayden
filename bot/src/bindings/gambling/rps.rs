use async_trait::async_trait;
use gambling::Commands;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{BotState, Error, Result};

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct RockPaperScissors;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for RockPaperScissors {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::rps::<BotState, Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_rps()
    }
}
