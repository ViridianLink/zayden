use std::sync::Arc;

use serenity::all::GuildId;
use songbird::id::ChannelId as SongbirdChannelId;
use songbird::{Call, Songbird};
use tokio::sync::Mutex;

use crate::error::{MusicError, Result};

pub async fn join<C>(
    songbird: &Arc<Songbird>,
    guild_id: GuildId,
    channel_id: C,
) -> Result<Arc<Mutex<Call>>>
where
    C: Into<SongbirdChannelId>,
{
    songbird
        .join(guild_id, channel_id)
        .await
        .map_err(|e| MusicError::Songbird(e.to_string()))
}

pub async fn leave(songbird: &Arc<Songbird>, guild_id: GuildId) -> Result<()> {
    songbird.leave(guild_id).await.map_err(|e| MusicError::Songbird(e.to_string()))
}

#[must_use]
pub fn get_call(
    songbird: &Arc<Songbird>,
    guild_id: GuildId,
) -> Option<Arc<Mutex<Call>>> {
    songbird.get(guild_id)
}
