use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use temp_voice::VoiceCommand;
use zayden_core::error::HandlerError;
use zayden_core::{InvocationCtx, ModuleCommand};

use super::VoiceChannelTable;
use crate::BotState;
use crate::sqlx_lib::GuildTable;

pub struct Voice;

#[async_trait]
impl ModuleCommand for Voice {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("voice")
    }

    fn definition(&self) -> CreateCommand<'static> {
        VoiceCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        VoiceCommand::run::<BotState, Postgres, GuildTable, VoiceChannelTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
