use std::fmt::Write;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use jiff::{SignedDuration, Timestamp};
use oauth2::{AuthorizationCode, TokenResponse};
use rand::RngExt;
use reqwest::StatusCode;
use serde::Deserialize;
use tower_cookies::Cookie;
use tower_cookies::cookie::SameSite;

use super::SESSION_COOKIE;
use crate::WebState;

const DISCORD_API: &str = "https://discord.com/api/v10";
const SESSION_TTL_HOURS: i64 = 24 * 7;
const STATE_TTL: Duration = Duration::from_mins(10);

#[derive(Deserialize)]
pub(super) struct DiscordAuthCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
}

fn error_redirect(frontend_url: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{frontend_url}/?error=auth_failed"))
        .body(Body::empty())
        .expect("static HTTP response headers are valid")
}

pub(super) async fn discord_auth_callback_handler(
    Query(query): Query<DiscordAuthCallback>,
    State(state): State<WebState>,
) -> impl IntoResponse {
    let frontend_url = state.frontend_url.as_str();

    match state.oauth_states.remove(&query.state) {
        Some((_, created_at)) if created_at.elapsed() <= STATE_TTL => {},
        _ => return error_redirect(frontend_url),
    }

    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_oauth)
        .await;

    let discord_access_token = match token_result {
        Ok(t) => t.access_token().secret().clone(),
        Err(_) => return error_redirect(frontend_url),
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
            Err(_) => return error_redirect(frontend_url),
        },
        _ => return error_redirect(frontend_url),
    };

    let discord_user_id: i64 = match discord_user.id.parse() {
        Ok(id) => id,
        Err(_) => return error_redirect(frontend_url),
    };

    // Generate a 32-byte cryptographically random session token encoded as 64 hex
    // chars.
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let session_token = bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });

    let expires_at = Timestamp::now()
        .saturating_add(SignedDuration::from_hours(SESSION_TTL_HOURS))
        .expect("7-day session TTL addition is within timestamp range");
    let expires_at = jiff_sqlx::Timestamp::from(expires_at);

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
    )]
    let insert_result = sqlx::query!(
        "INSERT INTO web_sessions \
             (token, discord_user_id, discord_access_token, expires_at) \
         VALUES ($1, $2, $3, $4)",
        &session_token,
        discord_user_id,
        &discord_access_token,
        expires_at as jiff_sqlx::Timestamp
    )
    .execute(&state.app.db)
    .await;

    if insert_result.is_err() {
        return error_redirect(frontend_url);
    }

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax);

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{frontend_url}/dashboard"))
        .header(header::SET_COOKIE, cookie.to_string())
        .body(Body::empty())
        .expect("static HTTP response headers are valid")
}
