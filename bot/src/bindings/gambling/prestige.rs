use std::borrow::Cow;

use async_trait::async_trait;
use gambling::Commands;
use gambling::components::PrestigeCustomId;
use serenity::all::CreateCommand;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

pub struct Prestige;

#[async_trait]
impl ModuleCommand for Prestige {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("prestige")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_prestige()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::prestige(cx.ctx, cx.interaction, &cx.app.db).await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Prestige {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("prestige"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        match cx.interaction.data.custom_id.parse::<PrestigeCustomId>()? {
            PrestigeCustomId::Confirm => {
                Commands::confirm_prestige(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                    cx.app.zayden_id,
                )
                .await?;
            },
            PrestigeCustomId::Cancel => {
                Commands::cancel_prestige(cx.ctx, cx.interaction).await?;
            },
        }

        Ok(())
    }
}
