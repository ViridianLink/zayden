use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sqlx::Row;
use tower_cookies::Cookies;
use tracing::{debug, warn};

use crate::WebState;
use crate::web::SESSION_COOKIE;

#[derive(Clone)]
pub(crate) struct AuthToken(pub(crate) String);

#[derive(Clone)]
pub(crate) struct AuthUser {
    pub(crate) id: String,
}

pub(crate) async fn require_auth(
    cookies: Cookies,
    State(state): State<WebState>,
    mut req: Request,
    next: Next,
) -> Response {
    let Some(session_token) =
        cookies.get(SESSION_COOKIE).map(|c| c.value().to_owned())
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    if let Some((discord_access_token, discord_user_id)) =
        state.session_cache.get(&session_token).await
    {
        debug!(user_id = discord_user_id, "authenticated request (cache hit)");
        req.extensions_mut().insert(AuthToken(discord_access_token));
        req.extensions_mut().insert(AuthUser { id: discord_user_id.to_string() });
        return next.run(req).await;
    }

    let row = sqlx::query(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
    )
    .bind(&session_token)
    .fetch_optional(&state.app.db)
    .await;

    let (discord_access_token, discord_user_id) = match row {
        Ok(Some(r)) => (
            r.get::<String, _>("discord_access_token"),
            r.get::<i64, _>("discord_user_id"),
        ),
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            warn!(?e, "Failed to look up session token");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    state
        .session_cache
        .insert(session_token, (discord_access_token.clone(), discord_user_id))
        .await;

    debug!(user_id = discord_user_id, "authenticated request");
    req.extensions_mut().insert(AuthToken(discord_access_token));
    req.extensions_mut().insert(AuthUser { id: discord_user_id.to_string() });
    next.run(req).await
}
