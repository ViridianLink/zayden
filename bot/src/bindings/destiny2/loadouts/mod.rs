use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::error::HandlerError;
use zayden_core::{InvocationCtx, ModuleCommand};

use crate::{BotState, ZAYDEN_TOKEN, zayden_token};

pub struct Loadout;

#[async_trait]
impl ModuleCommand for Loadout {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("builds")
    }

    fn definition(&self) -> CreateCommand<'static> {
        destiny2::loadouts::Loadout::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let zayden_token =
            ZAYDEN_TOKEN.get_or_init(|| zayden_token(&cx.app.db)).await;
        let options = cx.interaction.data.options();
        destiny2::loadouts::Loadout::run::<BotState>(
            cx.ctx,
            cx.interaction,
            options,
            zayden_token,
        )
        .await?;
        Ok(())
    }
}
