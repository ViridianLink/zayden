use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use topcoat::cookie::time::Duration;

use super::context::cookie_jar;
use super::cookie::{self, LOGIN_NEXT_COOKIE};
use super::error::AuthError;

const MAX_NEXT_LEN: usize = 512;
const NEXT_TTL_MINUTES: i64 = 10;

#[must_use]
pub fn safe_next(path: &str) -> Option<&str> {
    let safe = path.len() <= MAX_NEXT_LEN
        && path.starts_with('/')
        && !path.starts_with("//")
        && path.chars().all(|c| c.is_ascii_graphic() && c != '\\');
    safe.then_some(path)
}

pub fn remember_login_next(cx: &Cx, path: &str) -> Result<(), AuthError> {
    if let Some(next) = safe_next(path) {
        cookie_jar(cx)?.add(cookie::build(
            LOGIN_NEXT_COOKIE,
            next,
            Duration::minutes(NEXT_TTL_MINUTES),
        ));
    }
    Ok(())
}
