use std::sync::Arc;

use serenity::all::{ChannelId, GuildId};
use songbird::error::JoinError;
use songbird::{Call, Songbird};
use tokio::sync::Mutex;

pub async fn connect(
    manager: &Songbird,
    guild: GuildId,
    channel: ChannelId,
) -> Result<Arc<Mutex<Call>>, JoinError> {
    manager.join(guild, channel).await
}
