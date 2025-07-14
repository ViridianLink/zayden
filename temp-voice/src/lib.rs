pub mod commands;
mod error;
pub mod events;
pub mod guild_manager;
pub mod voice_channel_manager;

use std::collections::HashMap;
use std::time::Duration;

use serenity::all::{
    ChannelId, Guild, GuildChannel, GuildId, Http, LightMethod, Request, Route, UserId, VoiceState,
};

pub use commands::VoiceCommand;
pub use error::Error;
use error::Result;
pub use guild_manager::{TempVoiceGuildManager, TempVoiceRow};
pub use voice_channel_manager::{VoiceChannelManager, VoiceChannelRow};

#[derive(Debug)]
pub struct CachedState {
    pub channel_id: Option<ChannelId>,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

impl CachedState {
    pub fn new(channel_id: Option<ChannelId>, guild_id: GuildId, user_id: UserId) -> Self {
        Self {
            channel_id,
            guild_id,
            user_id,
        }
    }
}

impl From<&VoiceState> for CachedState {
    fn from(state: &VoiceState) -> Self {
        Self {
            channel_id: state.channel_id,
            guild_id: state.guild_id.unwrap(),
            user_id: state.user_id,
        }
    }
}

pub trait VoiceStateCache: Send + Sync + 'static {
    fn get(&self) -> &HashMap<UserId, CachedState>;

    fn get_mut(&mut self) -> &mut HashMap<UserId, CachedState>;

    fn guild_create(&mut self, guild: &Guild) {
        let cache = self.get_mut();

        guild
            .voice_states
            .iter()
            .filter(|state| state.channel_id.is_some())
            .for_each(|state| {
                cache.insert(
                    state.user_id,
                    CachedState::new(state.channel_id, guild.id, state.user_id),
                );
            })
    }

    fn update(&mut self, new: &VoiceState) -> Result<Option<CachedState>> {
        let cache = self.get_mut();

        let old = if new.channel_id.is_none() {
            cache.remove(&new.user_id)
        } else {
            cache.insert(new.user_id, new.into())
        };

        Ok(old)
    }
}

pub async fn get_voice_state(
    http: &Http,
    guild_id: GuildId,
    user_id: UserId,
) -> serenity::Result<VoiceState> {
    http.fire::<VoiceState>(Request::new(
        Route::GuildVoiceStates { guild_id, user_id },
        LightMethod::Get,
    ))
    .await
}

pub async fn delete_voice_channel_if_inactive(
    http: &Http,
    guild_id: GuildId,
    user_id: UserId,
    vc: &GuildChannel,
) -> bool {
    tokio::time::sleep(Duration::from_secs(60)).await;

    match get_voice_state(http, guild_id, user_id).await {
        Ok(voice_state) if voice_state.channel_id == Some(vc.id) => false,
        _ => {
            vc.delete(http, Some("Empty and inactive channel"))
                .await
                .unwrap();
            true
        }
    }
}
