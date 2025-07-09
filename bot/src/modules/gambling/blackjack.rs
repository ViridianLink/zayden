use async_trait::async_trait;
use gambling::Commands;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{Component, SlashCommand};

use crate::{Error, Result};

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct Blackjack;

#[async_trait]
impl SlashCommand<Error, Postgres> for Blackjack {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::blackjack::<Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(Commands::register_blackjack())
    }
}

#[async_trait]
impl Component<Error, Postgres> for Blackjack {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
        match interaction.data.custom_id.as_str() {
            "blackjack_hit" => gambling::components::Blackjack::hit::<
                Postgres,
                GoalsTable,
                EffectsTable,
                GameTable,
            >(ctx, interaction, pool)
            .await?,
            "blackjack_stand" => {
                gambling::components::Blackjack::stand::<
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }
            "blackjack_double" => {
                gambling::components::Blackjack::double::<
                    Postgres,
                    GamblingTable,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }

            id => unreachable!("Invalid custom_id: {id}"),
        }

        Ok(())
    }
}
