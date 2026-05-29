mod mw_auth;
mod routes_guild;
mod routes_kofi;
mod routes_login;

use axum::Router;
use axum::routing::get;
use routes_login::discord_auth_callback_handler;

const AUTH_TOKEN: &str = "auth-token";

pub use routes_kofi::kofi_webhook_handler;

use crate::AppState;
use crate::web::routes_guild::{channels, guild, settings, zayden};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/callback", get(discord_auth_callback_handler))
        .route("/guild/{id}", get(guild))
        .route("/guild/{id}/channels", get(channels))
        .route("/users/@me/guilds/{id}/member", get(zayden))
        .route("/guild/{id}/settings", get(settings))
}
