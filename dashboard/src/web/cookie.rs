use tower_cookies::Cookie;
use tower_cookies::cookie::time::Duration;
use tower_cookies::cookie::{CookieBuilder, SameSite};

pub(crate) const SESSION_COOKIE: &str = "session";
pub(crate) const OAUTH_STATE_COOKIE: &str = "oauth_state";

pub(crate) fn build(
    name: &'static str,
    value: impl Into<String>,
    max_age: Duration,
) -> CookieBuilder<'static> {
    Cookie::build((name, value.into()))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(max_age)
}
