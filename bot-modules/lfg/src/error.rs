use std::borrow::Cow;

use serenity::all::{Mentionable, UserId};
use zayden_core::Error as ZaydenError;
use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => ZaydenError::MissingGuildId.fmt(f),
            Self::MissingSetup => {
                write!(
                    f,
                    "Missing setup. If you are the owner, please run `/lfg setup` to set up the bot."
                )
            }
            Self::FireteamFull => write!(f, "Unable to join. Fireteam is full."),
            Self::PermissionDenied(id) => write!(
                f,
                "Permission denied. Only the owner ({}) can use this action.",
                id.mention()
            ),
            Self::InvalidDateTime(format) => {
                write!(f, "Bot currently only accepts {format} for dates and time.")
            }
            Self::TagRequired => {
                write!(
                    f,
                    "Couldn't create the post: the activity name didn't match any known category, so the required forum tag couldn't be applied. Please use a recognised activity name and try again."
                )
            }
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

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) => None,
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
