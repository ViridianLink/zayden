use serenity::all::{Mentionable, UserId};
use zayden_core::Error as ZaydenError;

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
                write!(f, "Invalid date time. Expected format: {format}")
            }
            Self::TagRequired => {
                write!(
                    f,
                    "Unable to parse Activity and apply necessary tags. Please fix the Activity field and use the edit button to update after creating the post."
                )
            }
            Self::InvalidChannel => write!(f, "Invalid LFG channel."),
            _ => write!(f, "Unhandled error"),
        }
    }
}

impl std::error::Error for Error {}

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
