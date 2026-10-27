use jiff::{SignedDuration, Timestamp};
use rand::RngExt;
use sqlx::PgPool;
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use topcoat::cookie::time::Duration;

use super::context::cookie_jar;
use super::cookie::{self, SESSION_COOKIE, SESSION_TTL_HOURS};
use super::error::AuthError;
use crate::util::hex_encode;

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
