use std::borrow::Cow;

use serenity::all::{ChannelId, Mentionable};
use zayden_core::error::{HandlerError, Respond};

#[expect(unreachable_pub, reason = "used through re-export in parent module")]
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum PermissionError {
    NotOwner,
    NotTrusted,
}

#[expect(clippy::error_impl_error, reason = "conventional error type naming")]
#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    MemberNotInVoiceChannel,
    OwnerInChannel,
    InvalidPassword,
    UserIsOwner,
    MaxChannels,
    MissingPermissions(PermissionError),
    ChannelNotFound(ChannelId),
    AdministratorRequired,
    IneligibleChannel,

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
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
            Self::MaxChannels => write!(
                f,
                "You have reached the maximum number of persistent channels."
            ),
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
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId
            | Self::MemberNotInVoiceChannel
            | Self::OwnerInChannel
            | Self::InvalidPassword
            | Self::UserIsOwner
            | Self::MaxChannels
            | Self::MissingPermissions(_)
            | Self::ChannelNotFound(_)
            | Self::AdministratorRequired
            | Self::IneligibleChannel => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::MissingGuildId
            | Self::MemberNotInVoiceChannel
            | Self::OwnerInChannel
            | Self::InvalidPassword
            | Self::UserIsOwner
            | Self::MaxChannels
            | Self::MissingPermissions(_)
            | Self::ChannelNotFound(_)
            | Self::AdministratorRequired
            | Self::IneligibleChannel => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<Error> for HandlerError {
    fn from(e: Error) -> Self {
        Self::from_respond(e)
    }
}
