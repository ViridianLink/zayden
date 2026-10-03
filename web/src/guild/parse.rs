//! Reads the string values settings forms submit.

use url::Url;
use zayden_app::config::ARCHIVE_NEVER;

use super::error::GuildError;

const MAX_LINK_LEN: usize = 200;

/// A snowflake column value: blank or unparsable input clears the setting.
#[must_use]
pub fn parse_id(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() { None } else { t.parse().ok() }
}

#[must_use]
pub fn parse_optional(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_owned()) }
}

/// A select or toggle value: only `"true"` is on.
#[must_use]
pub fn parse_flag(s: &str) -> bool {
    s.trim() == "true"
}

#[must_use]
pub fn opt_str(v: Option<i64>) -> Option<String> {
    v.map(|n| n.to_string())
}

/// A wiki base URL with its trailing `/` removed, or `None` when blank.
pub fn parse_wiki_url(s: &str) -> Result<Option<String>, GuildError> {
    let Some(raw) = parse_optional(s) else {
        return Ok(None);
    };

    let Ok(url) = Url::parse(raw.trim_end_matches('/')) else {
        return Err(GuildError::InvalidWikiUrl);
    };

    if !matches!(url.scheme(), "http" | "https") {
        return Err(GuildError::WikiUrlScheme);
    }

    Ok(Some(url.as_str().trim_end_matches('/').to_owned()))
}

/// A role snowflake. `u64::MAX` is no snowflake, so it is refused too.
pub fn parse_role(s: &str) -> Result<u64, GuildError> {
    parse_snowflake(s).ok_or(GuildError::InvalidRole)
}

/// A user snowflake. `u64::MAX` is no snowflake, so it is refused too.
pub fn parse_user(s: &str) -> Result<u64, GuildError> {
    parse_snowflake(s).ok_or(GuildError::InvalidUserId)
}

fn parse_snowflake(s: &str) -> Option<u64> {
    s.trim().parse::<u64>().ok().filter(|id| *id != u64::MAX)
}

/// A helper's link: an http(s) URL without credentials, at most 200
/// characters once normalized.
pub fn parse_link(s: &str) -> Result<String, GuildError> {
    let url = match Url::parse(s.trim()) {
        Ok(url) => url,
        Err(e) => return Err(GuildError::InvalidLink(e.to_string())),
    };

    if !matches!(url.scheme(), "http" | "https") {
        return Err(GuildError::LinkScheme);
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(GuildError::LinkCredentials);
    }

    let link = url.to_string();

    if link.len() > MAX_LINK_LEN {
        return Err(GuildError::LinkTooLong);
    }

    Ok(link)
}

/// The solved-thread archive delay: negative never archives, unparsable
/// input is 60 seconds.
#[must_use]
pub fn parse_archive_secs(s: &str) -> i32 {
    match s.trim().parse::<i32>() {
        Ok(n) if n < 0 => ARCHIVE_NEVER,
        Ok(n) => n,
        Err(_e) => 60,
    }
}

/// An idle or stale delay, clamped to the columns' one-hour floor and a
/// one-month ceiling.
#[must_use]
pub fn parse_idle_secs(s: &str, default: i32) -> i32 {
    s.trim().parse::<i32>().unwrap_or(default).clamp(3_600, 2_592_000)
}

#[must_use]
pub fn parse_max_partners(s: &str) -> i32 {
    s.trim().parse::<i32>().unwrap_or(1).max(1)
}

#[must_use]
pub fn parse_wiki_locale(s: &str) -> String {
    match s.trim() {
        "" => String::from("en"),
        locale => locale.to_owned(),
    }
}

#[must_use]
pub fn parse_max_results(s: &str) -> i32 {
    s.trim().parse().unwrap_or(5_i32).clamp(1, 25)
}

#[must_use]
pub fn parse_answer_max_tokens(s: &str) -> i32 {
    s.trim().parse().unwrap_or(500_i32).clamp(64, 4096)
}

#[must_use]
pub fn parse_answer_temperature(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.2_f32).clamp(0.0, 2.0)
}
