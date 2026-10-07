use sqlx::PgPool;
use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use twilight_model::user::CurrentUser;

use super::context::{cookie_jar, db_pool, try_web_state};
use super::cookie::SESSION_COOKIE;
use super::dto::SessionUser;
use super::error::AuthError;
use crate::state::{SessionCache, SessionIdentity, SessionUsersCache};

pub const LOGIN_PATH: &str = "/login";

pub async fn lookup_session(
    cache: Option<&SessionCache>,
    pool: &PgPool,
    token: &str,
) -> Result<Option<SessionIdentity>, sqlx::Error> {
    if let Some(cache) = cache
        && let Some(identity) = cache.get(token).await
    {
        return Ok(Some(identity));
    }

    let row = sqlx::query!(
        "SELECT discord_access_token, discord_user_id FROM web_sessions \
         WHERE token = $1 AND expires_at > now()",
        token,
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let identity = SessionIdentity {
        user_id: row.discord_user_id,
        access_token: row.discord_access_token,
    };

    if let Some(cache) = cache {
        cache.insert(token.to_owned(), identity.clone()).await;
    }

    Ok(Some(identity))
}

pub(super) async fn session_for_token(
    cx: &Cx,
    token: &str,
) -> Result<Option<SessionIdentity>, AuthError> {
    let pool = db_pool(cx)?;
    let cache = try_web_state(cx).map(|state| &state.sessions);
    Ok(lookup_session(cache, pool, token).await?)
}

pub async fn current_session_identity(
    cx: &Cx,
) -> Result<Option<SessionIdentity>, AuthError> {
    db_pool(cx)?;
    let Some(cookie) = cookie_jar(cx)?.get(SESSION_COOKIE) else {
        return Ok(None);
    };

    session_for_token(cx, cookie.value()).await
}

pub async fn check_session(cx: &Cx) -> Result<bool, AuthError> {
    Ok(current_session_identity(cx).await?.is_some())
}

pub async fn current_user_id(cx: &Cx) -> Result<i64, AuthError> {
    current_session_identity(cx)
        .await?
        .map(|identity| identity.user_id)
        .ok_or(AuthError::Unauthenticated)
}

pub async fn require_user(cx: &Cx) -> Result<SessionIdentity, AuthError> {
    current_session_identity(cx).await?.ok_or(AuthError::Unauthenticated)
}

pub async fn current_session_user(
    cx: &Cx,
) -> Result<Option<SessionUser>, AuthError> {
    let Some(identity) = current_session_identity(cx).await? else {
        return Ok(None);
    };

    let cache = try_web_state(cx).map(|state| &state.discord.users);
    Ok(lookup_session_user(cache, &identity).await)
}

pub async fn lookup_session_user(
    cache: Option<&SessionUsersCache>,
    identity: &SessionIdentity,
) -> Option<SessionUser> {
    if let Some(cache) = cache
        && let Some(user) = cache.get(&identity.user_id).await
    {
        return Some(user);
    }

    let user = match super::discord::bearer_client(&identity.access_token)
        .current_user()
        .await
    {
        Ok(response) => response.model().await,
        Err(e) => {
            tracing::warn!(error = ?e, "request to Discord /users/@me failed");
            return None;
        },
    };

    let user = match user {
        Ok(u) => session_user(u),
        Err(e) => {
            tracing::warn!(error = ?e, "failed to parse Discord /users/@me response");
            return None;
        },
    };

    if let Some(cache) = cache {
        cache.insert(identity.user_id, user.clone()).await;
    }

    Some(user)
}

pub(super) async fn remember_session_user(cx: &Cx, user_id: i64, user: CurrentUser) {
    if let Some(state) = try_web_state(cx) {
        state.discord.users.insert(user_id, session_user(user)).await;
    }
}

fn session_user(user: CurrentUser) -> SessionUser {
    SessionUser {
        id: user.id.to_string(),
        name: user.global_name.unwrap_or(user.name),
        avatar: user.avatar.map(|hash| hash.to_string()),
    }
}
