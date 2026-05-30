use serenity::all::{Context, VoiceState};
use sqlx::{PgPool, Postgres};
use temp_voice::VoiceStateCache;
use temp_voice::events::voice_state_update::{channel_creator, channel_deleter};
use tokio::sync::RwLock;

use super::VoiceChannelTable;
use crate::sqlx_lib::GuildTable;
use crate::{BotState, Result};

pub async fn run(ctx: &Context, pool: &PgPool, new: &VoiceState) -> Result<()> {
    let old = {
        let data = ctx.data::<RwLock<BotState>>();
        let mut data = data.write().await;
        data.update(new)?
    };

    futures::try_join!(
        channel_creator::<Postgres, GuildTable, VoiceChannelTable>(
            &ctx.http, pool, new
        ),
        channel_deleter::<BotState, Postgres, GuildTable, VoiceChannelTable>(
            ctx,
            pool,
            old.as_ref()
        ),
    )?;

    Ok(())
}
