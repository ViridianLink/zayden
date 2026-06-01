use std::borrow::Cow;

use serenity::all::{
    DiscordJsonError,
    ErrorResponse,
    HttpError,
    JsonErrorCode,
    StatusCode,
};

pub trait Respond: std::error::Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        None
    }
}

#[derive(Debug)]
pub enum HandlerError {
    /// A database error from sqlx.
    Database(sqlx::Error),
    /// A Discord API error from serenity.
    Discord(serenity::Error),
    /// An error originating from a module handler, with an optional
    /// user-visible message extracted from a [`Respond`] impl.
    Module {
        source: Box<dyn std::error::Error + Send + Sync>,
        user_message: Option<String>,
    },
}

impl HandlerError {
    pub fn new<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Module { source: Box::new(err), user_message: None }
    }

    pub fn from_respond<E>(err: E) -> Self
    where
        E: Respond + Send + Sync + 'static,
    {
        let user_message = err.user_message().map(Cow::into_owned);
        Self::Module { source: Box::new(err), user_message }
    }

    #[must_use]
    pub fn user_message(&self) -> Option<&str> {
        match self {
            Self::Database(_) | Self::Discord(_) => None,
            Self::Module { user_message, .. } => user_message.as_deref(),
        }
    }

    #[must_use]
    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync) {
        match self {
            Self::Database(e) => e,
            Self::Discord(e) => e,
            Self::Module { source, .. } => source.as_ref(),
        }
    }
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::Discord(e) => write!(f, "discord error: {e}"),
            Self::Module { source, .. } => source.fmt(f),
        }
    }
}

impl std::error::Error for HandlerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            Self::Discord(e) => Some(e),
            Self::Module { source, .. } => Some(source.as_ref()),
        }
    }
}

impl From<sqlx::Error> for HandlerError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<serenity::Error> for HandlerError {
    fn from(e: serenity::Error) -> Self {
        Self::Discord(e)
    }
}

#[derive(Debug)]
pub enum CoreError {
    MissingGuildId,
    NotInteractionAuthor,

    MessageConflict,

    Serenity(serenity::Error),
    // region: Sqlx
    Sqlx(sqlx::Error),
    // endregion
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => {
                write!(f, "This command can only be used within a server.")
            },
            Self::NotInteractionAuthor => {
                write!(f, "You are not the author of this interaction.")
            },
            Self::MessageConflict => write!(
                f,
                "Command is already awaiting interaction. Please respond to previous command first."
            ),

            Self::Serenity(serenity::Error::Http(
                HttpError::UnsuccessfulRequest(ErrorResponse {
                    status_code:
                        StatusCode::INTERNAL_SERVER_ERROR
                        | StatusCode::SERVICE_UNAVAILABLE,
                    ..
                }),
            )) => write!(
                f,
                "It looks like Discord is currently experiencing some server issues. Please try your request again shortly. If the problem persists, please contact OscarSix for more details."
            ),

            Self::Serenity(serenity::Error::Http(
                HttpError::UnsuccessfulRequest(ErrorResponse {
                    error: DiscordJsonError { code, .. },
                    ..
                }),
            )) => match *code {
                JsonErrorCode::UnknownChannel => {
                    write!(f, "Channel already deleted")
                },
                JsonErrorCode::UnknownMessage => {
                    write!(f, "Message was unexpectably deleted. Please try again.")
                },
                JsonErrorCode::UnknownWebhook => write!(f, "Unknown Webhook"),
                JsonErrorCode::UnknownInteraction => write!(
                    f,
                    "An error occurred while processing the interaction. Please try again."
                ),
                JsonErrorCode::MissingAccess => write!(
                    f,
                    "I don't have access to that resource. Please check my role or permissions, or ask a server admin for help."
                ),
                JsonErrorCode::LackPermissionsForAction => write!(
                    f,
                    "I'm missing permissions perform that action. Please contact a server admin to resolve this."
                ),
                JsonErrorCode::OperationOnArchivedThread => {
                    write!(f, "This thread has already been closed and archived.")
                },
                _ => write!(f, "serenity: {self:?}"),
            },
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),

            Self::Sqlx(sqlx::Error::PoolTimedOut) => write!(
                f,
                "An internal error occurred while accessing data. Please try again shortly."
            ),
            Self::Sqlx(sqlx::Error::ColumnDecode { index, source })
                if source.is::<sqlx::error::UnexpectedNullError>() =>
            {
                write!(
                    f,
                    "Unexpected null found at {index}, please contact OscarSix to resolve."
                )
            },
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::MissingGuildId
            | Self::NotInteractionAuthor
            | Self::MessageConflict => None,
        }
    }
}

impl Respond for CoreError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::MissingGuildId
            | Self::NotInteractionAuthor
            | Self::MessageConflict
            | Self::Serenity(serenity::Error::Http(
                HttpError::UnsuccessfulRequest(ErrorResponse {
                    status_code:
                        StatusCode::INTERNAL_SERVER_ERROR
                        | StatusCode::SERVICE_UNAVAILABLE,
                    ..
                }),
            )) => Some(Cow::Owned(self.to_string())),

            Self::Serenity(serenity::Error::Http(
                HttpError::UnsuccessfulRequest(ErrorResponse {
                    error: DiscordJsonError { code, .. },
                    ..
                }),
            )) => match *code {
                JsonErrorCode::UnknownChannel
                | JsonErrorCode::UnknownMessage
                | JsonErrorCode::UnknownWebhook
                | JsonErrorCode::UnknownInteraction
                | JsonErrorCode::MissingAccess
                | JsonErrorCode::LackPermissionsForAction
                | JsonErrorCode::OperationOnArchivedThread => {
                    Some(Cow::Owned(self.to_string()))
                },
                _ => None,
            },

            Self::Sqlx(sqlx::Error::PoolTimedOut) => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::Sqlx(sqlx::Error::ColumnDecode { source, .. })
                if source.is::<sqlx::error::UnexpectedNullError>() =>
            {
                Some(Cow::Owned(self.to_string()))
            },

            Self::Serenity(_) | Self::Sqlx(_) => None,
        }
    }
}
