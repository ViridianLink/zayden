use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use dashboard::server::auth::lookup_session;
use tower_cookies::Cookies;
use tracing::{debug, warn};

use crate::state::SessionState;
use crate::web::cookie::SESSION_COOKIE;

#[derive(Clone)]
pub(crate) struct AuthUser {
    pub(crate) id: String,
}

pub(crate) async fn require_auth(
    cookies: Cookies,
    State(session): State<SessionState>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(session_token) =
        cookies.get(SESSION_COOKIE).map(|c| c.value().to_owned())
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let identity =
        match lookup_session(Some(&session.cache), &session.app.db, &session_token)
            .await
        {
            Ok(Some(identity)) => identity,
            Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
            Err(e) => {
                warn!(?e, "Failed to look up session token");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            },
        };

    debug!(user_id = identity.user_id, "authenticated request");
    req.extensions_mut().insert(AuthUser { id: identity.user_id.to_string() });
    next.run(req).await
}
