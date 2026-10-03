use topcoat::router::error::{SeeOther, see_other};
use twilight_http::response::DeserializeBodyError;

use super::session::LOGIN_PATH;

pub const UNAUTHENTICATED: &str = "unauthenticated";
pub const FORBIDDEN: &str = "forbidden";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    #[error("unauthenticated")]
    Unauthenticated,
    #[error("forbidden")]
    Forbidden,
    #[error("invalid guild id")]
    InvalidGuildId,
    #[error("Zayden isn't in that server")]
    BotNotInGuild,
    #[error(
        "{bot} can't post in #{channel}: it needs View Channel and Send Messages \
         there. Allow them for the bot's role in the channel's permissions, then \
         save again."
    )]
    BotCannotPost { bot: String, channel: String },
    #[error(transparent)]
    ForeignId(#[from] ForeignIdError),
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Discord(String),
    #[error("{0}")]
    MissingContext(&'static str),
}

impl AuthError {
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        matches!(self, Self::Unauthenticated | Self::Forbidden)
    }

    pub fn redirect_unauthenticated(self) -> Result<Self, SeeOther> {
        if self == Self::Unauthenticated {
            return Err(see_other(LOGIN_PATH));
        }
        Ok(self)
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<twilight_http::Error> for AuthError {
    fn from(e: twilight_http::Error) -> Self {
        Self::Discord(e.to_string())
    }
}

impl From<DeserializeBodyError> for AuthError {
    fn from(e: DeserializeBodyError) -> Self {
        Self::Discord(e.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ForeignIdError {
    #[error("that channel is not in this server")]
    Channel,
    #[error("that role is not in this server")]
    Role,
}
