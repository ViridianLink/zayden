mod routes_guild;
mod routes_kofi;
mod routes_login;

pub(crate) const AUTH_TOKEN: &str = "auth-token";

use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use routes_login::discord_auth_callback_handler;

use crate::WebState;
use crate::middleware::auth::require_auth;
use crate::middleware::guild_permission::require_guild_permission;
use crate::middleware::tier::require_pro;
use crate::web::routes_guild::{channels, guild, settings, zayden};

pub(crate) fn routes(state: WebState) -> Router<WebState> {
    // Free guild routes: auth + MANAGE_GUILD only.
    let guild_routes = Router::new()
        .route("/guild/{id}", get(guild))
        .route("/guild/{id}/channels", get(channels))
        .route_layer(from_fn_with_state(state.clone(), require_guild_permission));

    // Premium guild routes: auth + MANAGE_GUILD + Pro tier.
    let pro_guild_routes = Router::new()
        .route("/guild/{id}/settings", get(settings))
        .route_layer(from_fn_with_state(state.clone(), require_pro))
        .route_layer(from_fn_with_state(state.clone(), require_guild_permission));

    let protected = Router::new()
        .route("/users/@me/guilds/{id}/member", get(zayden))
        .merge(guild_routes)
        .merge(pro_guild_routes)
        .route_layer(from_fn_with_state(state, require_auth));

    Router::new()
        .route("/auth/callback", get(discord_auth_callback_handler))
        .route("/webhooks/kofi", post(routes_kofi::kofi_webhook_handler))
        .merge(protected)
}
