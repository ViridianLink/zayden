use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::StatusCode;
use topcoat::router::error::see_other;
use topcoat::router::response::{IntoResponse, Response};

use super::error::RequestRejection;

pub(super) fn redirect(cx: &Cx, location: &str) -> Result<Response> {
    see_other(location).into_response(cx)
}

pub(super) fn acknowledge(cx: &Cx) -> Result<Response> {
    StatusCode::OK.into_response(cx)
}

pub(super) fn reject(cx: &Cx, rejection: &RequestRejection) -> Result<Response> {
    (rejection.status(), rejection.to_string()).into_response(cx)
}
