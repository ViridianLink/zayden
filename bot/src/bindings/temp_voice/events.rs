use std::sync::Arc;

use serenity::all::{Context, VoiceState};
use sqlx::PgPool;
use temp_voice::events::voice_state_update::{channel_creator, channel_deleter};
use tokio::sync::RwLock;

use crate::{BotState, Result};

pub async fn run(ctx: &Context, pool: &PgPool, new: &VoiceState) -> Result<()> {
    let voice_states =
        Arc::clone(&ctx.data::<RwLock<BotState>>().read().await.voice_states);
    let old = voice_states.update(new)?;

    futures::try_join!(
        channel_creator(&ctx.http, pool, new),
        channel_deleter(ctx, pool, &voice_states, old.as_ref()),
    )?;

    Ok(())
}
