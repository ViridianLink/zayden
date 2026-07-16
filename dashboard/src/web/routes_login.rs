use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::{SignedDuration, Timestamp};
use oauth2::{AuthorizationCode, TokenResponse};
use rand::RngExt;
use serde::Deserialize;
use tower_cookies::cookie::SameSite;
use tower_cookies::{Cookie, Cookies};

use super::{OAUTH_STATE_COOKIE, SESSION_COOKIE};
use crate::WebState;

const SESSION_TTL_HOURS: i64 = 24 * 7;

#[derive(Deserialize)]
pub(super) struct DiscordAuthCallback {
    code: String,
    state: String,
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
    let mut removal = Cookie::from(OAUTH_STATE_COOKIE);
    removal.set_path("/");
    cookies.remove(removal);
    if !matches!(&cookie_state, Some(s) if *s == query.state && !s.is_empty()) {
        tracing::warn!(
            has_cookie = cookie_state.is_some(),
            "OAuth callback rejected: state cookie missing or does not match the returned state"
        );
        return error_redirect();
    }

    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_oauth)
        .await;

    let discord_access_token = match token_result {
        Ok(t) => t.access_token().secret().clone(),
        Err(e) => {
            tracing::warn!(error = ?e, "OAuth token exchange with Discord failed (check DISCORD_CLIENT_SECRET and that redirect_uri matches the portal registration exactly)");
            return error_redirect();
        },
    };

    let discord_user = twilight_http::Client::builder()
        .token(format!("Bearer {discord_access_token}"))
        .build()
        .current_user()
        .await;

    let discord_user = match discord_user {
        Ok(r) => match r.model().await {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(error = ?e, "failed to parse Discord /users/@me response");
                return error_redirect();
            },
        },
        Err(e) => {
            tracing::warn!(error = ?e, "request to Discord /users/@me failed");
            return error_redirect();
        },
    };

    let discord_user_id: i64 = discord_user.id.get().cast_signed();

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let session_token = dashboard::util::hex_encode(&bytes);

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

    if let Err(e) = insert_result {
        tracing::warn!(error = ?e, "failed to insert web_sessions row on login");
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
