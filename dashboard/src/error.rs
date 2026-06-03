use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use tracing::error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Sqlx(sqlx::Error),
    Discord(reqwest::Error),
    NotFound,
    Upstream(String),
    BadRequest,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Sqlx(e) => {
                error!(?e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "database error"})),
                )
                    .into_response()
            },
            Self::Discord(e) => {
                error!(?e, "Discord API error");
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"error": "Discord API error"})),
                )
                    .into_response()
            },
            Self::NotFound => {
                (StatusCode::NOT_FOUND, Json(json!({"error": "not found"})))
                    .into_response()
            },
            Self::Upstream(msg) => {
                error!(msg, "upstream error");
                (StatusCode::BAD_GATEWAY, Json(json!({"error": msg})))
                    .into_response()
            },
            Self::BadRequest => {
                (StatusCode::BAD_REQUEST, Json(json!({"error": "bad request"})))
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

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Discord(e)
    }
}
