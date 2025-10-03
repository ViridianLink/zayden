use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use oauth2::{AuthorizationCode, TokenResponse};
use reqwest::StatusCode;
use serde::Deserialize;
use tower_cookies::Cookie;

use crate::web::AUTH_TOKEN;
use crate::{AppState, FRONTEND_URL};

#[derive(Deserialize)]
pub struct DiscordAuthCallback {
    code: String,
    #[allow(dead_code)]
    state: String, // You should use and verify the CSRF token in a real app
}

pub async fn discord_auth_callback_handler(
    Query(query): Query<DiscordAuthCallback>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_client)
        .await;

    if let Ok(token) = token_result {
        let token = token.access_token().secret();

        let cookie = Cookie::build((AUTH_TOKEN, token.clone())).path("/");

        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(header::LOCATION, format!("{FRONTEND_URL}/dashboard"))
            .header(header::SET_COOKIE, cookie.to_string())
    } else {
        Response::builder().status(StatusCode::SEE_OTHER).header(
            header::LOCATION,
            format!("{FRONTEND_URL}/?error=auth_failed"),
        )
    }
    .body(String::new())
    .unwrap()
}

// You would now use this token to fetch user data from the Discord API.
// Then, you'd create a session (e.g., using a JWT or a session cookie)
// and store the user's info.
// For this example, we'll just redirect to the dashboard.

// This is where you would set a session cookie.
// Using a library like `axum-sessions` or `tower-cookies` is recommended.
