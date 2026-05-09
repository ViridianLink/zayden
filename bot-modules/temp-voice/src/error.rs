use std::borrow::Cow;

use serenity::all::{ChannelId, Mentionable};
use zayden_core::error::Respond;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum PermissionError {
    NotOwner,
    NotTrusted,
}

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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
            Error::MemberNotInVoiceChannel => {
                write!(
                    f,
                    "You must be in a voice channel or use the `channel` option to specify a channel to use this command."
                )
            }
            Error::OwnerInChannel => {
                write!(
                    f,
                    "Cannot use this command while the channel owner is present."
                )
            }
            Error::InvalidPassword => write!(f, "Invalid channel password."),
            Error::UserIsOwner => write!(f, "You are already the owner of this channel."),
            Error::MaxChannels => write!(
                f,
                "You have reached the maximum number of persistent channels."
            ),
            Error::MissingPermissions(PermissionError::NotOwner) => {
                write!(f, "Only the channel owner can use this command.")
            }
            Error::MissingPermissions(PermissionError::NotTrusted) => {
                write!(f, "You must be trusted to use this command.")
            }
            Error::ChannelNotFound(id) => write!(
                f,
                "Channel not found: {}\nTry using `/voice claim` to claim the channel.",
                id.mention()
            ),
            Error::AdministratorRequired => {
                write!(f, "You must be an administrator to run this command.")
            }
            Error::IneligibleChannel => {
                write!(f, "This channel isn't eligible for voice commands.")
            }
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) => None,
            Self::Sqlx(_) => None,
            _ => Some(Cow::Owned(self.to_string())),
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
