use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::message_metadata;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use super::{EffectsTable, GamblingTable, GameTable, GoalsTable};
use crate::BotState;

pub struct Blackjack;

#[async_trait]
impl ModuleCommand for Blackjack {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("blackjack")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_blackjack()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::blackjack::<
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
impl ModuleComponent for Blackjack {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("blackjack"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let metadata = message_metadata(&cx.interaction.message)?;

        if cx.interaction.user != metadata.user {
            debug!();
            return Ok(());
        }

        match cx.interaction.data.custom_id.as_str() {
            "blackjack_hit" => {
                gambling::components::Blackjack::hit::<
                    BotState,
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(cx.ctx, cx.interaction, &cx.app.db)
                .await?;
            },
            "blackjack_stand" => {
                gambling::components::Blackjack::stand::<
                    BotState,
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(cx.ctx, cx.interaction, &cx.app.db)
                .await?;
            },
            "blackjack_double" => {
                gambling::components::Blackjack::double::<
                    BotState,
                    Postgres,
                    GamblingTable,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(cx.ctx, cx.interaction, &cx.app.db)
                .await?;
            },
            "blackjack_split" => {
                gambling::components::Blackjack::split::<
                    BotState,
                    Postgres,
                    GamblingTable,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(cx.ctx, cx.interaction, &cx.app.db)
                .await?;
            },
            "blackjack_surrender" => {
                gambling::components::Blackjack::surrender::<
                    BotState,
                    Postgres,
                    GoalsTable,
                    EffectsTable,
                    GameTable,
                >(cx.ctx, cx.interaction, &cx.app.db)
                .await?;
            },
            _ => (),
        }

        Ok(())
    }
}
