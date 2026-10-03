use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::response::{IntoResponse, Response};
use topcoat::router::{StatusCode, route};

use crate::auth::web_state;

#[route(GET "/invite")]
pub(crate) async fn invite(cx: &Cx) -> Result<Response> {
    web_state(cx)?.urls.invite.as_deref().map_or_else(
        || StatusCode::NOT_FOUND.into_response(cx),
        |url| see_other(url).into_response(cx),
    )
}
