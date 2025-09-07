mod mw_auth;
mod routes_dashboard;
mod routes_login;
mod routes_test;

use axum::Router;
use axum::routing::get;
use routes_login::discord_auth_callback_handler;

const AUTH_TOKEN: &str = "auth-token";

use crate::AppState;
use crate::web::routes_dashboard::dashboard;
use crate::web::routes_test::{add_cookie, list_cookies};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/callback", get(discord_auth_callback_handler))
        .route("/dashboard", get(dashboard))
        .route("/add_cookie", get(add_cookie))
        .route("/list_cookies", get(list_cookies))
}
