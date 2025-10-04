mod mw_auth;
mod routes_guild;
mod routes_login;

use axum::Router;
use axum::routing::get;
use routes_login::discord_auth_callback_handler;

const AUTH_TOKEN: &str = "auth-token";

use crate::AppState;
use crate::web::routes_guild::{channels, guild};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/callback", get(discord_auth_callback_handler))
        .route("/guild/{id}", get(guild))
        .route("/guild/{id}/channels", get(channels))
}
