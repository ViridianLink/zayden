use std::borrow::Cow;

use serenity::all::{DiscordJsonError, ErrorResponse, HttpError, JsonErrorCode, StatusCode};

pub trait Respond: std::error::Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        None
    }
}

/// A type-erased error returned by all module handler traits.
///
/// Wraps any `std::error::Error + Send + Sync` and carries an optional
/// user-visible message extracted from the original error's [`Respond`] impl.
#[derive(Debug)]
pub struct HandlerError {
    inner: Box<dyn std::error::Error + Send + Sync>,
    user_message: Option<String>,
}

impl HandlerError {
    pub fn new<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            inner: Box::new(err),
            user_message: None,
        }
    }

    pub fn from_respond<E>(err: E) -> Self
    where
        E: Respond + Send + Sync + 'static,
    {
        let user_message = err.user_message().map(|s| s.into_owned());
        Self {
            inner: Box::new(err),
            user_message,
        }
    }

    pub fn user_message(&self) -> Option<&str> {
        self.user_message.as_deref()
    }

    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync) {
        &*self.inner
    }
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for HandlerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    NotInteractionAuthor,

    MessageConflict,

    Serenity(serenity::Error),
    //region: Sqlx
    Sqlx(sqlx::Error),
    //endregion
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingGuildId => write!(f, "This command can only be used within a server."),
            Error::NotInteractionAuthor => write!(f, "You are not the author of this interaction."),
            Error::MessageConflict => write!(
                f,
                "Command is already awaiting interaction. Please respond to previous command first."
            ),

            Error::Serenity(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                serenity::all::ErrorResponse {
                    status_code: StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE,
                    ..
                },
            ))) => write!(
                f,
                "It looks like Discord is currently experiencing some server issues. Please try your request again shortly. If the problem persists, please contact OscarSix for more details."
            ),

            Self::Serenity(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error: DiscordJsonError { code, .. },
                    ..
                },
            ))) => match *code {
                JsonErrorCode::UnknownChannel => write!(f, "Channel already deleted"),
                JsonErrorCode::UnknownMessage => {
                    write!(f, "Message was unexpectably deleted. Please try again.")
                }
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
                }
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
            }
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
            Self::MissingGuildId | Self::NotInteractionAuthor | Self::MessageConflict => {
                Some(Cow::Owned(self.to_string()))
            }

            Self::Serenity(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    status_code: StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE,
                    ..
                },
            ))) => Some(Cow::Owned(self.to_string())),

            Self::Serenity(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error: DiscordJsonError { code, .. },
                    ..
                },
            ))) => match *code {
                JsonErrorCode::UnknownChannel
                | JsonErrorCode::UnknownMessage
                | JsonErrorCode::UnknownWebhook
                | JsonErrorCode::UnknownInteraction
                | JsonErrorCode::MissingAccess
                | JsonErrorCode::LackPermissionsForAction
                | JsonErrorCode::OperationOnArchivedThread => Some(Cow::Owned(self.to_string())),
                _ => None,
            },
            Self::Serenity(_) => None,

            Self::Sqlx(sqlx::Error::PoolTimedOut) => Some(Cow::Owned(self.to_string())),
            Self::Sqlx(sqlx::Error::ColumnDecode { source, .. })
                if source.is::<sqlx::error::UnexpectedNullError>() =>
            {
                Some(Cow::Owned(self.to_string()))
            }
            Self::Sqlx(_) => None,
        }
    }
}
