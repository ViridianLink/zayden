use std::borrow::Cow;

use serenity::all::{Mentionable, UserId};
use zayden_core::Error as ZaydenError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, LfgError>;

#[expect(
    clippy::error_impl_error,
    reason = "conventional error type name in domain crate"
)]
#[derive(Debug)]
pub enum LfgError {
    MissingGuildId,
    MissingSetup,
    FireteamFull,
    PermissionDenied(UserId),
    InvalidDateTime(String),
    TagRequired,
    InvalidChannel,
    ThreadNotFound,

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for LfgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => ZaydenError::MissingGuildId.fmt(f),
            Self::MissingSetup => {
                write!(
                    f,
                    "Missing setup. If you are the owner, please run `/lfg setup` to set up the bot."
                )
            },
            Self::FireteamFull => write!(f, "Unable to join. Fireteam is full."),
            Self::PermissionDenied(id) => write!(
                f,
                "Permission denied. Only the owner ({}) can use this action.",
                id.mention()
            ),
            Self::InvalidDateTime(format) => {
                write!(f, "Bot currently only accepts {format} for dates and time.")
            },
            Self::TagRequired => {
                write!(
                    f,
                    "Couldn't create the post: the activity name didn't match any known category, so the required forum tag couldn't be applied. Please use a recognised activity name and try again."
                )
            },
            Self::InvalidChannel => write!(f, "Invalid LFG channel."),
            Self::ThreadNotFound => write!(
                f,
                "Unable to find the requested thread. Make sure you're running this command in an LFG thread or supplying one with the `thread:` option."
            ),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for LfgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId
            | Self::MissingSetup
            | Self::FireteamFull
            | Self::PermissionDenied(_)
            | Self::InvalidDateTime(_)
            | Self::TagRequired
            | Self::InvalidChannel
            | Self::ThreadNotFound => None,
        }
    }
}

impl Respond for LfgError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::MissingGuildId
            | Self::MissingSetup
            | Self::FireteamFull
            | Self::PermissionDenied(_)
            | Self::InvalidDateTime(_)
            | Self::TagRequired
            | Self::InvalidChannel
            | Self::ThreadNotFound => Some(Cow::Owned(self.to_string())),
        }
    }
}

impl From<serenity::Error> for LfgError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for LfgError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}
