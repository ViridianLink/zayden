use serenity::all::{DiscordJsonError, ErrorResponse, HttpError, JsonErrorCode, StatusCode};

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
                _ => unimplemented!("Unhandled Discord error: {self:?}"),
            },
            Self::Serenity(e) => unimplemented!("Unhandled Serenity error: {e:?}"),

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
            Self::Sqlx(e) => unimplemented!("Unhandled Sqlx error: {e:?}"),
        }
    }
}
