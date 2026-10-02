use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookie, SameSite};

pub const SESSION_COOKIE: &str = "session";
pub const OAUTH_STATE_COOKIE: &str = "oauth_state";
/// Lifetime of both the session cookie and its `web_sessions` row.
pub const SESSION_TTL_HOURS: i64 = 24 * 7;

/// Builds every cookie the dashboard sets. `max_age` is required so no cookie
/// outlives, or is outlived by, the server state it refers to.
#[must_use]
pub fn build(
    name: &'static str,
    value: impl Into<String>,
    max_age: Duration,
) -> Cookie<'static> {
    Cookie::build((name, value.into()))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .build()
}
