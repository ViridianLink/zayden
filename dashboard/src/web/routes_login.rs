use std::fmt::Write;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use jiff::SignedDuration;
use jiff_sqlx::Timestamp as SqlxTimestamp;
use oauth2::{AuthorizationCode, TokenResponse};
use rand::RngCore;
use rand::rngs::OsRng;
use reqwest::StatusCode;
use serde::Deserialize;
use tower_cookies::Cookie;
use tower_cookies::cookie::SameSite;

use super::SESSION_COOKIE;
use crate::{FRONTEND_URL, WebState};

const DISCORD_API: &str = "https://discord.com/api/v10";
const SESSION_TTL_HOURS: i64 = 24 * 7;

#[derive(Deserialize)]
pub(super) struct DiscordAuthCallback {
    code: String,
    _state: String, // TODO: verify the CSRF token (M11.2.4)
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
}

fn error_redirect() -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{FRONTEND_URL}/?error=auth_failed"))
        .body(Body::empty())
        .expect("static HTTP response headers are valid")
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

    let discord_access_token = match token_result {
        Ok(t) => t.access_token().secret().clone(),
        Err(_) => return error_redirect(),
    };

    let user_resp = state
        .app
        .http
        .get(format!("{DISCORD_API}/users/@me"))
        .header("Authorization", format!("Bearer {discord_access_token}"))
        .send()
        .await;

    let discord_user: DiscordUser = match user_resp {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(u) => u,
            Err(_) => return error_redirect(),
        },
        _ => return error_redirect(),
    };

    let discord_user_id: i64 = match discord_user.id.parse() {
        Ok(id) => id,
        Err(_) => return error_redirect(),
    };

    // Generate a 32-byte cryptographically random session token encoded as 64 hex
    // chars.
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let session_token = bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });

    let expires_at: SqlxTimestamp = jiff::Timestamp::now()
        .saturating_add(SignedDuration::from_hours(SESSION_TTL_HOURS))
        .expect("7-day session TTL addition is within timestamp range")
        .into();

    let insert_result = sqlx::query(
        "INSERT INTO web_sessions (token, discord_user_id, discord_access_token, expires_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&session_token)
    .bind(discord_user_id)
    .bind(&discord_access_token)
    .bind(expires_at)
    .execute(&state.app.db)
    .await;

    if insert_result.is_err() {
        return error_redirect();
    }

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax);

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{FRONTEND_URL}/dashboard"))
        .header(header::SET_COOKIE, cookie.to_string())
        .body(Body::empty())
        .expect("static HTTP response headers are valid")
}
