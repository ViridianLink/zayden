use std::fmt::Write;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use jiff::{SignedDuration, Timestamp};
use oauth2::{AuthorizationCode, TokenResponse};
use rand::RngExt;
use reqwest::StatusCode;
use serde::Deserialize;
use tower_cookies::cookie::SameSite;
use tower_cookies::{Cookie, Cookies};
use twilight_model::id::Id;
use twilight_model::id::marker::UserMarker;

use super::{OAUTH_STATE_COOKIE, SESSION_COOKIE};
use crate::WebState;

const DISCORD_API: &str = "https://discord.com/api/v10";
const SESSION_TTL_HOURS: i64 = 24 * 7;

#[derive(Deserialize)]
pub(super) struct DiscordAuthCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: Id<UserMarker>,
}

fn error_redirect() -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login?error=auth_failed")
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(super) async fn discord_auth_callback_handler(
    Query(query): Query<DiscordAuthCallback>,
    cookies: Cookies,
    State(state): State<WebState>,
) -> impl IntoResponse {
    let cookie_state = cookies.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_owned());
    cookies.remove(Cookie::from(OAUTH_STATE_COOKIE));
    match cookie_state {
        Some(s) if s == query.state && !s.is_empty() => {},
        _ => return error_redirect(),
    }

    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_oauth)
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

    let discord_user_id: i64 = discord_user.id.get().cast_signed();

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let session_token = bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });

    let expires_at = Timestamp::now()
        .saturating_add(SignedDuration::from_hours(SESSION_TTL_HOURS))
        .unwrap_or(Timestamp::MAX);
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
        return error_redirect();
    }

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax);

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/guilds")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(super) async fn logout_handler(
    cookies: Cookies,
    State(state): State<WebState>,
) -> impl IntoResponse {
    if let Some(token) = cookies.get(SESSION_COOKIE).map(|c| c.value().to_owned()) {
        if let Err(e) =
            sqlx::query!("DELETE FROM web_sessions WHERE token = $1", token)
                .execute(&state.app.db)
                .await
        {
            tracing::warn!(?e, "failed to delete session row on logout");
        }
        state.session_cache.invalidate(&token).await;
    }

    let cleared = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(tower_cookies::cookie::time::Duration::ZERO);

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login")
        .header(header::SET_COOKIE, cleared.to_string())
        .body(Body::empty())
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
