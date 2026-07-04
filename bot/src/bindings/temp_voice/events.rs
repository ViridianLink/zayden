use std::sync::Arc;

use serenity::all::{Context, VoiceState};
use sqlx::{PgPool, Postgres};
use temp_voice::GuildTable;
use temp_voice::events::voice_state_update::{channel_creator, channel_deleter};
use tokio::sync::RwLock;

use super::VoiceChannelTable;
use crate::{BotState, Result};

pub async fn run(ctx: &Context, pool: &PgPool, new: &VoiceState) -> Result<()> {
    let voice_states =
        Arc::clone(&ctx.data::<RwLock<BotState>>().read().await.voice_states);
    let old = voice_states.update(new)?;

    futures::try_join!(
        channel_creator::<Postgres, GuildTable, VoiceChannelTable>(
            &ctx.http, pool, new
        ),
        channel_deleter::<Postgres, GuildTable, VoiceChannelTable>(
            ctx,
            pool,
            &voice_states,
            old.as_ref()
        ),
    )?;

    Ok(())
}
