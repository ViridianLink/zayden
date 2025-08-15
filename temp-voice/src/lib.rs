pub mod commands;
mod error;
pub mod events;
pub mod guild_manager;
pub mod voice_channel_manager;

use std::collections::HashMap;
use std::time::Duration;

use serenity::all::{
    ChannelId, DiscordJsonError, ErrorResponse, Guild, GuildChannel, GuildId, Http, HttpError,
    JsonErrorCode, PermissionOverwrite, PermissionOverwriteType, Permissions, UserId, VoiceState,
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

pub async fn delete_voice_channel_if_inactive(
    http: &Http,
    guild_id: GuildId,
    user_id: UserId,
    vc: &GuildChannel,
) -> bool {
    tokio::time::sleep(Duration::from_secs(60)).await;

    match guild_id.get_user_voice_state(http, user_id).await {
        Ok(voice_state) if voice_state.channel_id == Some(vc.id) => false,
        _ => {
            match vc.delete(http, Some("Empty and inactive channel")).await {
                Ok(_)
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::UnknownChannel,
                            ..
                        },
                    ..
                }))) => {}
                Err(e) => panic!("{e:?}"),
            }

            true
        }
    }
}

pub fn owner_perms(user: UserId) -> PermissionOverwrite {
    PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::MANAGE_CHANNELS
            | Permissions::CONNECT
            | Permissions::SPEAK
            | Permissions::USE_SOUNDBOARD
            | Permissions::USE_EXTERNAL_SOUNDS
            | Permissions::USE_VAD
            | Permissions::PRIORITY_SPEAKER
            | Permissions::MUTE_MEMBERS
            | Permissions::DEAFEN_MEMBERS
            | Permissions::MOVE_MEMBERS
            | Permissions::SET_VOICE_CHANNEL_STATUS
            | Permissions::SEND_MESSAGES
            | Permissions::EMBED_LINKS
            | Permissions::ATTACH_FILES
            | Permissions::ADD_REACTIONS
            | Permissions::USE_EXTERNAL_EMOJIS
            | Permissions::USE_EXTERNAL_STICKERS
            | Permissions::MANAGE_MESSAGES
            | Permissions::READ_MESSAGE_HISTORY
            | Permissions::SEND_TTS_MESSAGES
            | Permissions::SEND_VOICE_MESSAGES
            | Permissions::SEND_POLLS
            | Permissions::USE_APPLICATION_COMMANDS
            | Permissions::USE_EMBEDDED_ACTIVITIES
            | Permissions::USE_EXTERNAL_APPS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user),
    }
}
