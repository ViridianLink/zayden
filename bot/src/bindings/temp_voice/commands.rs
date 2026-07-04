use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use sqlx::Postgres;
use temp_voice::{GuildTable, VoiceCommand};
use tokio::sync::RwLock;
use zayden_core::error::HandlerError;
use zayden_core::{InvocationCtx, ModuleCommand};

use super::VoiceChannelTable;
use crate::BotState;

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
        let voice_states =
            Arc::clone(&cx.ctx.data::<RwLock<BotState>>().read().await.voice_states);

        VoiceCommand::run::<Postgres, GuildTable, VoiceChannelTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
            &voice_states,
        )
        .await?;
        Ok(())
    }
}
