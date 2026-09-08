//! The Patreon OAuth handler redirects out of the Axum layer into the Leptos
//! router, so nothing at compile time ties the URL it builds to the routes the
//! router declares. Every terminal state of the flow goes through
//! `settings_href`, which makes this the seam worth pinning.
#![cfg(feature = "ssr")]

use dashboard::ui::nav::settings_href;

/// `app.rs` declares its routes through the `path!` macro, so the route table
/// is only observable as source text from outside the Leptos runtime.
const ROUTER: &str = include_str!("../src/app.rs");

#[test]
fn patreon_redirects_at_the_settings_section_route() {
    assert_eq!(
        settings_href("428610928000876544", "patreon").as_deref(),
        Some("/guild/428610928000876544/settings/patreon")
    );
}

#[test]
fn the_router_declares_the_route_settings_href_builds() {
    assert!(ROUTER.contains(r#"path!("/guild/:id")"#));
    assert!(ROUTER.contains(r#"path!("/settings/:section")"#));
}

#[test]
fn every_section_resolves_under_the_same_prefix() {
    for slug in ["general", "ai", "family", "honeypot", "lfg", "music", "support"] {
        assert_eq!(
            settings_href("1", slug).as_deref(),
            Some(format!("/guild/1/settings/{slug}").as_str())
        );
    }
}

#[test]
fn an_unknown_slug_does_not_invent_a_route() {
    assert_eq!(settings_href("1", "patron"), None);
    assert_eq!(settings_href("1", "greetings"), None);
}
