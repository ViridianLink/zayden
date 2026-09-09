pub(crate) mod cookie;
mod routes_kofi;
mod routes_login;
mod routes_patreon;

use axum::Router;
use axum::extract::FromRef;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use routes_login::{discord_auth_callback_handler, logout_handler};
use sqlx::PgPool;
use tracing::warn;

use crate::middleware::auth::require_auth;
use crate::state::{SessionState, WebState};

pub(crate) fn routes(state: &WebState) -> Router<WebState> {
    let protected = Router::new()
        .route("/kofi/link", post(routes_kofi::kofi_link_handler))
        .route("/patreon/connect", get(routes_patreon::patreon_connect_handler))
        .route("/patreon/callback", get(routes_patreon::patreon_callback_handler))
        .route_layer(from_fn_with_state(
            SessionState::from_ref(state),
            require_auth,
        ));

    Router::new()
        .route("/auth/callback", get(discord_auth_callback_handler))
        .route("/logout", get(logout_handler))
        .route("/webhooks/kofi", post(routes_kofi::kofi_webhook_handler))
        .route("/webhooks/patreon", post(routes_patreon::patreon_webhook_handler))
        .merge(protected)
}

pub(crate) async fn prune_expired_sessions(pool: &PgPool) {
    if let Err(e) =
        sqlx::query!("DELETE FROM web_sessions WHERE expires_at <= now()")
            .execute(pool)
            .await
    {
        warn!(?e, "failed to prune expired web_sessions");
    }
}
