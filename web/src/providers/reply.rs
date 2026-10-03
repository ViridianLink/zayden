use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::StatusCode;
use topcoat::router::error::see_other;
use topcoat::router::response::{IntoResponse, Response};

use super::error::RequestRejection;

/// A `303` to `location` with an empty body.
pub(super) fn redirect(cx: &Cx, location: &str) -> Result<Response> {
    see_other(location).into_response(cx)
}

/// A `200` with an empty body, which is how every webhook answers.
pub(super) fn acknowledge(cx: &Cx) -> Result<Response> {
    StatusCode::OK.into_response(cx)
}

pub(super) fn reject(cx: &Cx, rejection: &RequestRejection) -> Result<Response> {
    (rejection.status(), rejection.to_string()).into_response(cx)
}
