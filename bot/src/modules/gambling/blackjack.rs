use async_trait::async_trait;
use gambling::Commands;
use serenity::all::{
    CommandInteraction, ComponentInteraction, Context, CreateCommand, MessageInteractionMetadata,
    ResolvedOption,
};
use sqlx::{PgPool, Postgres};
use zayden_core::{ApplicationCommand, Component};

use crate::{BotState, Error, Result};

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};

pub struct Blackjack;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Blackjack {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::blackjack::<BotState, Postgres, GamblingTable, GoalsTable, EffectsTable, GameTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_blackjack()
    }
}

#[async_trait]
impl Component<Error, Postgres> for Blackjack {
    async fn run(ctx: &Context, interaction: &ComponentInteraction, pool: &PgPool) -> Result<()> {
        let Some(MessageInteractionMetadata::Command(metadata)) =
            interaction.message.interaction_metadata.as_deref()
        else {
            unreachable!("Message must be created from an command")
        };

        if interaction.user != metadata.user {
            return Ok(());
        };

        match interaction.data.custom_id.as_str() {
            "blackjack_hit" => {
                gambling::components::Blackjack::hit::<
                    BotState,
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }
            "blackjack_stand" => {
                gambling::components::Blackjack::stand::<
                    BotState,
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }
            "blackjack_double" => {
                gambling::components::Blackjack::double::<
                    BotState,
                    Postgres,
                    GamblingTable,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }
            "blackjack_split" => {
                gambling::components::Blackjack::split::<
                    BotState,
                    Postgres,
                    GamblingTable,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(ctx, interaction, pool)
                .await?
            }
            "blackjack_surrender" => {
                gambling::components::Blackjack::surrender::<
                    BotState,
                    Postgres,
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
