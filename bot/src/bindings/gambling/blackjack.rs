use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use serenity::all::CreateCommand;
use tracing::debug;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::message_metadata;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

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
        Commands::blackjack::<BotState>(cx.ctx, cx.interaction, options, &cx.app.db)
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
            debug!(
                user_id = %cx.interaction.user.id,
                owner_id = %metadata.user.id,
                "user does not own this blackjack message; ignoring"
            );
            return Ok(());
        }

        match cx.interaction.data.custom_id.as_str() {
            "blackjack_hit" => {
                gambling::components::Blackjack::hit::<BotState>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                )
                .await?;
            },
            "blackjack_stand" => {
                gambling::components::Blackjack::stand::<BotState>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                )
                .await?;
            },
            "blackjack_double" => {
                gambling::components::Blackjack::double::<BotState>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                )
                .await?;
            },
            "blackjack_split" => {
                gambling::components::Blackjack::split::<BotState>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                )
                .await?;
            },
            "blackjack_surrender" => {
                gambling::components::Blackjack::surrender::<BotState>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                )
                .await?;
            },
            _ => (),
        }

        Ok(())
    }
}
