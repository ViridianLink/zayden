use rand::RngExt;
use topcoat::context::Cx;
use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookie, Cookies};

use crate::auth::{AuthError, build_cookie, cookie_jar};
use crate::nav;
use crate::util::hex_encode;

const STATE_TTL_MINUTES: i64 = 10;

pub(super) fn settings_url(
    guild_id: &str,
    slug: &str,
    param: &str,
    outcome_key: &str,
) -> String {
    let base = nav::settings_href(guild_id, slug)
        .unwrap_or_else(|| format!("/guild/{guild_id}/settings"));

    format!("{base}?{param}={outcome_key}")
}

pub(super) fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    hex_encode(&bytes)
}

pub(super) fn oauth_state(nonce: &str, guild: &str) -> String {
    format!("{nonce}.{guild}")
}

pub(super) fn remember_nonce(
    cx: &Cx,
    cookie_name: &'static str,
    nonce: String,
) -> Result<(), AuthError> {
    cookie_jar(cx)?.add(build_cookie(
        cookie_name,
        nonce,
        Duration::minutes(STATE_TTL_MINUTES),
    ));
    Ok(())
}

pub(super) fn take_nonce(
    cx: &Cx,
    cookie_name: &'static str,
) -> Result<Option<String>, AuthError> {
    let jar = cookie_jar(cx)?;
    let nonce = jar.get(cookie_name).map(|c| c.value().to_owned());
    let mut removal = Cookie::from(cookie_name);
    removal.set_path("/");
    jar.remove(removal);
    Ok(nonce)
}

pub(super) fn split_state(state: Option<&str>) -> Option<(&str, &str)> {
    state.and_then(|s| s.split_once('.'))
}

pub(super) fn nonce_matches(cookie: Option<&str>, returned: &str) -> bool {
    matches!(cookie, Some(n) if n == returned && !n.is_empty())
}
