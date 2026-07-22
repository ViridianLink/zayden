use std::borrow::Cow;

use serenity::all::{ChannelId, Mentionable};
use zayden_core::CoreError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, TempVoiceError>;

#[derive(Debug)]
pub enum PermissionError {
    NotOwner,
    NotTrusted,
}

#[derive(Debug)]
pub enum TempVoiceError {
    MissingGuildId,
    MemberNotInVoiceChannel,
    OwnerInChannel,
    InvalidPassword,
    UserIsOwner,
    ClaimFailed,
    MaxChannels,
    InvalidNumber,
    MissingPermissions(PermissionError),
    ChannelNotFound(ChannelId),
    AdministratorRequired,
    IneligibleChannel,

    Internal(String),

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for TempVoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => CoreError::MissingGuildId.fmt(f),
            Self::MemberNotInVoiceChannel => {
                write!(
                    f,
                    "You must be in a voice channel or use the `channel` option to specify a channel to use this command."
                )
            },
            Self::OwnerInChannel => {
                write!(
                    f,
                    "Cannot use this command while the channel owner is present."
                )
            },
            Self::InvalidPassword => write!(f, "Invalid channel password."),
            Self::UserIsOwner => {
                write!(f, "You are already the owner of this channel.")
            },
            Self::ClaimFailed => write!(
                f,
                "This channel was just claimed by someone else. Please try again."
            ),
            Self::MaxChannels => write!(
                f,
                "You have reached the maximum number of persistent channels."
            ),
            Self::InvalidNumber => write!(f, "Please enter a valid number."),
            Self::MissingPermissions(PermissionError::NotOwner) => {
                write!(f, "Only the channel owner can use this command.")
            },
            Self::MissingPermissions(PermissionError::NotTrusted) => {
                write!(f, "You must be trusted to use this command.")
            },
            Self::ChannelNotFound(id) => write!(
                f,
                "Channel not found: {}\nTry using `/voice claim` to claim the channel.",
                id.mention()
            ),
            Self::AdministratorRequired => {
                write!(f, "You must be an administrator to run this command.")
            },
            Self::IneligibleChannel => {
                write!(f, "This channel isn't eligible for voice commands.")
            },
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for TempVoiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId
            | Self::MemberNotInVoiceChannel
            | Self::OwnerInChannel
            | Self::InvalidPassword
            | Self::UserIsOwner
            | Self::ClaimFailed
            | Self::MaxChannels
            | Self::InvalidNumber
            | Self::MissingPermissions(_)
            | Self::ChannelNotFound(_)
            | Self::AdministratorRequired
            | Self::IneligibleChannel
            | Self::Internal(_) => None,
        }
    }
}

impl Respond for TempVoiceError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) | Self::Internal(_) => None,
            Self::MissingGuildId
            | Self::MemberNotInVoiceChannel
            | Self::OwnerInChannel
            | Self::InvalidPassword
            | Self::UserIsOwner
            | Self::ClaimFailed
            | Self::MaxChannels
            | Self::InvalidNumber
            | Self::MissingPermissions(_)
            | Self::ChannelNotFound(_)
            | Self::AdministratorRequired
            | Self::IneligibleChannel => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<serenity::Error> for TempVoiceError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for TempVoiceError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<TempVoiceError> for HandlerError {
    fn from(e: TempVoiceError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for TempVoiceError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Module { .. } => Self::MissingGuildId,
        }
    }
}
