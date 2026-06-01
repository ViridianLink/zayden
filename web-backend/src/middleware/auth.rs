use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tower_cookies::Cookies;
use tracing::{debug, warn};

use crate::WebState;
use crate::web::AUTH_TOKEN;

const DISCORD_API: &str = "https://discord.com/api/v10";

/// Raw Discord Bearer token stored in request extensions by [`require_auth`]
/// for downstream middleware that needs to make user-scoped Discord API calls.
#[derive(Clone)]
pub(crate) struct AuthToken(pub(crate) String);

/// Verified Discord user attached to authenticated requests via [`Request`]
/// extensions.
#[derive(Clone, Deserialize)]
pub(crate) struct AuthUser {
    pub(crate) id: String,
    pub(crate) username: String,
}

/// Axum middleware that validates the Discord access token stored in the
/// `auth-token` cookie.
///
/// On success, inserts [`AuthUser`] into request extensions. Returns 401 if the
/// cookie is absent or the token is rejected by Discord's API.
pub(crate) async fn require_auth(
    cookies: Cookies,
    State(state): State<WebState>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(token) = cookies.get(AUTH_TOKEN).map(|c| c.value().to_owned()) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let resp = match state
        .app
        .http
        .get(format!("{DISCORD_API}/users/@me"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            warn!(?e, "Discord /users/@me request failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    if !resp.status().is_success() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let user = match resp.json::<AuthUser>().await {
        Ok(u) => u,
        Err(e) => {
            warn!(?e, "Failed to parse Discord user response");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    debug!(user_id = %user.id, username = %user.username, "authenticated request");
    req.extensions_mut().insert(AuthToken(token));
    req.extensions_mut().insert(user);
    next.run(req).await
}
