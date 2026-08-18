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

pub struct HigherLower;

#[async_trait]
impl ModuleCommand for HigherLower {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("higherorlower")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_higher_lower()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::higher_lower::<BotState>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for HigherLower {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("hol"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        let metadata = message_metadata(&cx.interaction.message)?;

        if cx.interaction.user != metadata.user {
            debug!(
                user_id = %cx.interaction.user.id,
                owner_id = %metadata.user.id,
                "user does not own this higher/lower message; ignoring"
            );
            return Ok(());
        }

        gambling::components::HigherLower::run_components::<BotState>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
