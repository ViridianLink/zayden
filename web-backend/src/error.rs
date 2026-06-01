use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Sqlx(e) => {
                error!(?e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "UNHANDLED_SERVER_ERROR")
                    .into_response()
            },
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}
