//! Deriving a wiki's endpoints from the one URL the dashboard collects.
//!
//! The dashboard stores the site origin only. Everything else - the GraphQL
//! endpoint, reader-facing article links, and the source view the module falls
//! back to when GraphQL refuses page source - is derived here, so a typo in one
//! field cannot leave the three disagreeing.

use ticket::wiki::WikiConfig;
use zayden_app::config::{FaqSettingsRow, SettingsRow};

fn row(url: Option<&str>, enabled: bool) -> FaqSettingsRow {
    let mut row = FaqSettingsRow::empty(1);
    row.enabled = enabled;
    row.wiki_url = url.map(str::to_owned);
    row
}

fn config(url: &str) -> Option<WikiConfig> {
    WikiConfig::from_settings(&row(Some(url), true)).ok().flatten()
}

#[test]
fn a_disabled_guild_has_no_config() {
    let built =
        WikiConfig::from_settings(&row(Some("https://wiki.example.com"), false));

    assert!(matches!(built, Ok(None)));
}

#[test]
fn a_guild_with_no_url_has_no_config() {
    assert!(matches!(WikiConfig::from_settings(&row(None, true)), Ok(None)));
    assert!(matches!(WikiConfig::from_settings(&row(Some("   "), true)), Ok(None)));
}

#[test]
fn a_url_without_a_scheme_is_rejected() {
    assert!(
        WikiConfig::from_settings(&row(Some("wiki.example.com"), true)).is_err()
    );
}

/// `file://` parses, so the scheme has to be checked separately from parsing.
#[test]
fn a_non_http_scheme_is_rejected() {
    assert!(
        WikiConfig::from_settings(&row(Some("file:///etc/passwd"), true)).is_err()
    );
}

#[test]
fn endpoints_are_derived_from_the_origin() {
    let config = config("https://wiki.example.com").expect("config builds");

    assert_eq!(
        config.graphql_endpoint().as_str(),
        "https://wiki.example.com/graphql"
    );
    assert_eq!(
        config.article_url("Docker").map(|u| u.to_string()).ok().as_deref(),
        Some("https://wiki.example.com/en/Docker")
    );
    assert_eq!(
        config.source_url("Docker").map(|u| u.to_string()).ok().as_deref(),
        Some("https://wiki.example.com/s/en/Docker")
    );
}

/// The dashboard trims it, but a stored trailing slash must not double up.
#[test]
fn a_trailing_slash_does_not_double_up() {
    let config = config("https://wiki.example.com/").expect("config builds");

    assert_eq!(
        config.graphql_endpoint().as_str(),
        "https://wiki.example.com/graphql"
    );
}

#[test]
fn locale_defaults_to_english_and_is_used_in_paths() {
    let mut row = row(Some("https://wiki.example.com"), true);
    row.wiki_locale = String::from("  ");

    let config =
        WikiConfig::from_settings(&row).ok().flatten().expect("config builds");

    assert_eq!(config.locale(), "en");

    row.wiki_locale = String::from("de");
    let config =
        WikiConfig::from_settings(&row).ok().flatten().expect("config builds");

    assert_eq!(
        config.article_url("Docker").map(|u| u.to_string()).ok().as_deref(),
        Some("https://wiki.example.com/de/Docker")
    );
}

#[test]
fn max_results_is_clamped_into_range() {
    let mut row = row(Some("https://wiki.example.com"), true);

    row.max_results = 0;
    assert_eq!(
        WikiConfig::from_settings(&row).ok().flatten().map(|c| c.max_results()),
        Some(1)
    );

    row.max_results = 500;
    assert_eq!(
        WikiConfig::from_settings(&row).ok().flatten().map(|c| c.max_results()),
        Some(25)
    );
}

#[test]
fn a_blank_api_key_is_treated_as_absent() {
    let mut row = row(Some("https://wiki.example.com"), true);
    row.wiki_api_key = Some(String::from("   "));

    let config =
        WikiConfig::from_settings(&row).ok().flatten().expect("config builds");

    assert!(config.api_key().is_none());
}
