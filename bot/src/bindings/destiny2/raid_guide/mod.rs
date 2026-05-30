use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::error::HandlerError;
use zayden_core::{InvocationCtx, ModuleCommand};

pub struct RaidGuide;

#[async_trait]
impl ModuleCommand for RaidGuide {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("raidguide")
    }

    fn definition(&self) -> CreateCommand<'static> {
        destiny2::raid_guides::RaidGuide::<0>::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        destiny2::raid_guides::RaidGuide::<0>::run(&cx.ctx.http, cx.interaction)
            .await;
        Ok(())
    }
}
