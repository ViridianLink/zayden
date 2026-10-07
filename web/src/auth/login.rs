use jiff::{SignedDuration, Timestamp};
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use rand::RngExt;
use sqlx::PgPool;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookie, Cookies};
use topcoat::router::error::{SeeOther, see_other};
use topcoat::router::{query_params, route};

use super::context::{cookie_jar, web_state};
use super::cookie::{self, OAUTH_STATE_COOKIE, SESSION_COOKIE, SESSION_TTL_HOURS};
use super::discord::bearer_client;
use super::error::AuthError;
use super::session::{LOGIN_PATH, remember_session_user};
use crate::util::hex_encode;

const OAUTH_STATE_TTL: std::time::Duration = std::time::Duration::from_mins(10);
const AUTH_FAILED_PATH: &str = "/login?error=auth_failed";
const SIGNED_IN_PATH: &str = "/guilds";

#[query_params(error = bad_request)]
pub(crate) struct DiscordAuthCallback {
    code: String,
    state: String,
}

#[route(GET "/auth/discord")]
pub(crate) async fn login_handler(cx: &Cx) -> Result<SeeOther> {
    let oauth = &web_state(cx)?.oauth;
    let (auth_url, csrf_token) = oauth
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scopes([
            Scope::new("identify".to_string()),
            Scope::new("guilds".to_string()),
            Scope::new("email".to_string()),
            Scope::new("applications.commands.permissions.update".to_string()),
        ])
        .url();

    let max_age = Duration::seconds(OAUTH_STATE_TTL.as_secs().cast_signed());
    cookie_jar(cx)?.add(cookie::build(
        OAUTH_STATE_COOKIE,
        csrf_token.secret().clone(),
        max_age,
    ));

    Ok(see_other(auth_url.as_str()))
}

#[route(GET "/auth/callback")]
pub(crate) async fn discord_auth_callback_handler(cx: &Cx) -> Result<SeeOther> {
    let query = query_params::<DiscordAuthCallback>(cx)?;
    let state = web_state(cx)?;

    let jar = cookie_jar(cx)?;
    let cookie_state = jar.get(OAUTH_STATE_COOKIE).map(|c| c.value().to_owned());
    let mut removal = Cookie::from(OAUTH_STATE_COOKIE);
    removal.set_path("/");
    jar.remove(removal);
    if !matches!(&cookie_state, Some(s) if *s == query.state && !s.is_empty()) {
        tracing::warn!(
            has_cookie = cookie_state.is_some(),
            "OAuth callback rejected: state cookie missing or does not match the returned state"
        );
        return Ok(see_other(AUTH_FAILED_PATH));
    }

    let token_result = state
        .oauth
        .client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(&state.oauth.http)
        .await;

    let discord_access_token = match token_result {
        Ok(t) => t.access_token().secret().clone(),
        Err(e) => {
            tracing::warn!(error = ?e, "OAuth token exchange with Discord failed (check DISCORD_CLIENT_SECRET and that redirect_uri matches the portal registration exactly)");
            return Ok(see_other(AUTH_FAILED_PATH));
        },
    };

    let discord_user = match bearer_client(&discord_access_token)
        .current_user()
        .await
    {
        Ok(r) => match r.model().await {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(error = ?e, "failed to parse Discord /users/@me response");
                return Ok(see_other(AUTH_FAILED_PATH));
            },
        },
        Err(e) => {
            tracing::warn!(error = ?e, "request to Discord /users/@me failed");
            return Ok(see_other(AUTH_FAILED_PATH));
        },
    };

    let discord_user_id: i64 = discord_user.id.get().cast_signed();

    if let Err(e) =
        start_session(cx, &state.app.db, discord_user_id, &discord_access_token)
            .await
    {
        tracing::warn!(error = ?e, "failed to insert web_sessions row on login");
        return Ok(see_other(AUTH_FAILED_PATH));
    }

    remember_session_user(cx, discord_user_id, discord_user).await;

    Ok(see_other(SIGNED_IN_PATH))
}

pub async fn start_session(
    cx: &Cx,
    pool: &PgPool,
    discord_user_id: i64,
    discord_access_token: &str,
) -> Result<(), AuthError> {
    let jar = cookie_jar(cx)?;

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let session_token = hex_encode(&bytes);

    let expires_at = Timestamp::now()
        .saturating_add(SignedDuration::from_hours(SESSION_TTL_HOURS))
        .unwrap_or(Timestamp::MAX);
    let expires_at = jiff_sqlx::Timestamp::from(expires_at);

    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    sqlx::query!(
        "INSERT INTO web_sessions \
             (token, discord_user_id, discord_access_token, expires_at) \
         VALUES ($1, $2, $3, $4)",
        &session_token,
        discord_user_id,
        discord_access_token,
        expires_at as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    jar.add(cookie::build(
        SESSION_COOKIE,
        session_token,
        Duration::hours(SESSION_TTL_HOURS),
    ));

    Ok(())
}

#[route(GET "/logout")]
pub(crate) async fn logout_handler(cx: &Cx) -> Result<SeeOther> {
    let state = web_state(cx)?;
    let jar = cookie_jar(cx)?;

    if let Some(token) = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()) {
        if let Err(e) =
            sqlx::query!("DELETE FROM web_sessions WHERE token = $1", token)
                .execute(&state.app.db)
                .await
        {
            tracing::warn!(?e, "failed to delete session row on logout");
        }
        state.sessions.invalidate(&token).await;
    }

    jar.add(cookie::build(SESSION_COOKIE, "", Duration::ZERO));

    Ok(see_other(LOGIN_PATH))
}
