use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use topcoat::router::response::IntoResponse;
use topcoat::router::{Body, LayerFn, LayerFuture, Next, StatusCode};

use super::context::cookie_jar;
use super::cookie::SESSION_COOKIE;
use super::session::session_for_token;

/// A layer that admits only requests carrying a live session to the routes
/// under `path`. Others get an empty `401`, or an empty `500` when the
/// session store fails.
#[must_use]
pub fn require_auth(path: &'static str) -> LayerFn {
    LayerFn::new(Some(path), require_auth_handler)
}

fn require_auth_handler<'a>(
    cx: &'a Cx,
    body: Body,
    next: Next<'a>,
) -> LayerFuture<'a> {
    Box::pin(async move {
        let jar = match cookie_jar(cx) {
            Ok(jar) => jar,
            Err(e) => {
                tracing::warn!(?e, "Failed to read the session cookie");
                return (StatusCode::INTERNAL_SERVER_ERROR, ()).into_response(cx);
            },
        };
        let Some(token) = jar.get(SESSION_COOKIE) else {
            return (StatusCode::UNAUTHORIZED, ()).into_response(cx);
        };

        let identity = match session_for_token(cx, token.value()).await {
            Ok(Some(identity)) => identity,
            Ok(None) => return (StatusCode::UNAUTHORIZED, ()).into_response(cx),
            Err(e) => {
                tracing::warn!(?e, "Failed to look up session token");
                return (StatusCode::INTERNAL_SERVER_ERROR, ()).into_response(cx);
            },
        };

        tracing::debug!(user_id = identity.user_id, "authenticated request");
        next.run(cx, body).await
    })
}
