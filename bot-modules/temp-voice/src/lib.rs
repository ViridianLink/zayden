pub mod actions;
pub mod commands;
pub mod error;
pub mod events;
pub mod guild_manager;
pub mod voice_channel_manager;

use std::time::Duration;

pub use commands::VoiceCommand;
use dashmap::DashMap;
pub use error::{Result, TempVoiceError};
pub use guild_manager::{GuildTable, TempVoiceGuildManager, TempVoiceRow};
use serenity::all::{
    ChannelId,
    DiscordJsonError,
    ErrorResponse,
    Guild,
    GuildChannel,
    GuildId,
    Http,
    HttpError,
    JsonErrorCode,
    PermissionOverwrite,
    PermissionOverwriteType,
    Permissions,
    UserId,
    VoiceState,
};
pub use voice_channel_manager::{VoiceChannelManager, VoiceChannelRow};

#[derive(Debug, Clone, Copy)]
pub struct CachedState {
    pub channel_id: Option<ChannelId>,
    pub guild_id: GuildId,
    pub user_id: UserId,
}

impl CachedState {
    #[must_use]
    pub const fn new(
        channel_id: Option<ChannelId>,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Self {
        Self { channel_id, guild_id, user_id }
    }
}

impl TryFrom<&VoiceState> for CachedState {
    type Error = TempVoiceError;

    fn try_from(state: &VoiceState) -> Result<Self> {
        Ok(Self {
            channel_id: state.channel_id,
            guild_id: state.guild_id.ok_or(TempVoiceError::MissingGuildId)?,
            user_id: state.user_id,
        })
    }
}

#[derive(Default)]
pub struct VoiceStateCache {
    states: DashMap<UserId, CachedState>,
}

impl VoiceStateCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn guild_create(&self, guild: &Guild) {
        for state in
            guild.voice_states.iter().filter(|state| state.channel_id.is_some())
        {
            self.states.insert(
                state.user_id,
                CachedState::new(state.channel_id, guild.id, state.user_id),
            );
        }
    }

    pub fn update(&self, new: &VoiceState) -> Result<Option<CachedState>> {
        let old = if new.channel_id.is_none() {
            self.states.remove(&new.user_id).map(|(_, state)| state)
        } else {
            self.states.insert(new.user_id, CachedState::try_from(new)?)
        };

        Ok(old)
    }

    #[must_use]
    pub fn current_channel(&self, user_id: UserId) -> Option<ChannelId> {
        self.states.get(&user_id).and_then(|state| state.channel_id)
    }

    #[must_use]
    pub fn occupants(&self, channel_id: ChannelId) -> Vec<UserId> {
        self.states
            .iter()
            .filter(|entry| entry.value().channel_id == Some(channel_id))
            .map(|entry| *entry.key())
            .collect()
    }
}

pub async fn delete_voice_channel_if_inactive(
    http: &Http,
    guild_id: GuildId,
    user_id: UserId,
    vc: &GuildChannel,
) -> bool {
    tokio::time::sleep(Duration::from_mins(1)).await;

    match guild_id.get_user_voice_state(http, user_id).await {
        Ok(voice_state) if voice_state.channel_id == Some(vc.id) => false,
        _ => {
            match vc.delete(http, Some("Empty and inactive channel")).await {
                Ok(_)
                | Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                    ErrorResponse {
                        error:
                            DiscordJsonError {
                                code: JsonErrorCode::UnknownChannel, ..
                            },
                        ..
                    },
                ))) => {},
                Err(e) => tracing::error!(
                    error = ?e,
                    channel_id = %vc.id,
                    guild_id = %guild_id,
                    "delete_voice_channel_if_inactive: vc.delete failed",
                ),
            }

            true
        },
    }
}

#[must_use]
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
