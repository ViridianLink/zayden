use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use oauth2::{AuthorizationCode, TokenResponse};
use reqwest::StatusCode;
use serde::Deserialize;
use tower_cookies::Cookie;

use super::AUTH_TOKEN;
use crate::{FRONTEND_URL, WebState};

#[derive(Deserialize)]
pub(super) struct DiscordAuthCallback {
    code: String,
    _state: String, // TODO: verify the CSRF token
}

pub(super) async fn discord_auth_callback_handler(
    Query(query): Query<DiscordAuthCallback>,
    State(state): State<WebState>,
) -> impl IntoResponse {
    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.app.http)
        .await;

    token_result
        .map_or_else(
            |_| {
                Response::builder().status(StatusCode::SEE_OTHER).header(
                    header::LOCATION,
                    format!("{FRONTEND_URL}/?error=auth_failed"),
                )
            },
            |token| {
                let token = token.access_token().secret();
                let cookie = Cookie::build((AUTH_TOKEN, token.clone())).path("/");
                Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header(
                        header::LOCATION,
                        format!("{FRONTEND_URL}/dashboard#token={token}"),
                    )
                    .header(header::SET_COOKIE, cookie.to_string())
            },
        )
        .body(String::new())
        .expect("static HTTP response headers are valid")
}
