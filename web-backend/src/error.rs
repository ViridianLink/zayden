use axum::http::StatusCode;
use axum::response::IntoResponse;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "UNHANDLED_SERVER_ERROR").into_response()
    }
}
