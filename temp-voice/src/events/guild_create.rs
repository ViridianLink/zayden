use serenity::all::{Context, Guild};
use tokio::sync::RwLock;

use crate::{CachedState, VoiceStateCache};

pub async fn guild_create<Data: VoiceStateCache>(ctx: &Context, guild: &Guild) {
    let data = ctx.data::<RwLock<Data>>();
    let mut data = data.write().await;
    let cache = data.get_mut();

    for state in guild
        .voice_states
        .iter()
        .filter(|state| state.channel_id.is_some())
    {
        cache.insert(
            state.user_id,
            CachedState::new(state.channel_id, guild.id, state.user_id),
        );
    }
}
