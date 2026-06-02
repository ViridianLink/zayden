use axum::Extension;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tracing::warn;
use zayden_app::entitlement::types::{EntitlementScope, Tier};

use crate::WebState;
use crate::middleware::auth::AuthUser;

/// Axum middleware that requires [`Tier::Pro`] (or higher) for the
/// authenticated user or their guild before the request proceeds.
///
/// Reads [`AuthUser`] from request extensions (set by the outer
/// [`super::auth::require_auth`] layer) and the guild id from the URI path
/// segment at index 2 (i.e. `/guild/{id}[/...]`).
///
/// Returns **402 Payment Required** when the entitlement check fails.
pub(crate) async fn require_pro(
    State(state): State<WebState>,
    Extension(user): Extension<AuthUser>,
    req: Request,
    next: Next,
) -> Response {
    let Ok(user_id) = user.id.parse::<u64>() else {
        warn!(user_id = %user.id, "AuthUser.id is not a valid Discord snowflake");
        return StatusCode::BAD_REQUEST.into_response();
    };

    let Some(guild_id) = guild_id_from_path(req.uri().path()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let scope = EntitlementScope::UserInGuild(user_id, guild_id);
    if !state.app.entitlements.allows(scope, Tier::Pro).await {
        return (
            StatusCode::PAYMENT_REQUIRED,
            "A Pro subscription is required to access this feature.",
        )
            .into_response();
    }

    next.run(req).await
}

fn guild_id_from_path(path: &str) -> Option<u64> {
    path.split('/').nth(2)?.parse().ok()
}
