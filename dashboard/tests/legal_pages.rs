//! Google's OAuth verification for the YouTube connection reads these pages:
//! both must be routed, linked from the public homepage, reachable without a
//! session, and carry the statements the YouTube API policies require. The
//! router and views are only observable as source text outside the Leptos
//! runtime, so that is what these tests pin.

const ROUTER: &str = include_str!("../src/app.rs");
const PUBLIC_LAYOUT: &str = include_str!("../src/ui/components/public_layout.rs");
const LEGAL_LAYOUT: &str = include_str!("../src/ui/components/legal.rs");
const LANDING: &str = include_str!("../src/ui/pages/landing.rs");
const PRIVACY: &str = include_str!("../src/ui/pages/privacy.rs");
const TERMS: &str = include_str!("../src/ui/pages/terms.rs");
const APP_LAYOUT: &str = include_str!("../src/ui/components/layout.rs");
const LOGIN: &str = include_str!("../src/ui/pages/login.rs");
const NOT_FOUND: &str = include_str!("../src/ui/pages/not_found.rs");

#[test]
fn the_router_declares_the_privacy_route() {
    assert!(ROUTER.contains(r#"<Route path=path!("/privacy") view=PrivacyPage/>"#));
}

#[test]
fn the_router_declares_the_terms_route() {
    assert!(ROUTER.contains(r#"<Route path=path!("/terms") view=TermsPage/>"#));
}

#[test]
fn the_public_footer_links_both_pages() {
    let (_, footer) = PUBLIC_LAYOUT.split_once("fn Footer()").unwrap_or_default();

    assert!(footer.contains(r#"<a href="/privacy">"Privacy Policy"</a>"#));
    assert!(footer.contains(r#"<a href="/terms">"Terms of Service"</a>"#));
    assert!(PUBLIC_LAYOUT.contains("<Footer/>"));
}

#[test]
fn the_legal_links_point_at_both_pages() {
    let (_, links) = LEGAL_LAYOUT.split_once("fn LegalLinks()").unwrap_or_default();

    assert!(links.contains(r#"<a href="/privacy">"Privacy Policy"</a>"#));
    assert!(links.contains(r#"<a href="/terms">"Terms of Service"</a>"#));
}

/// Pages outside the public layout have no footer, so they carry the links
/// themselves: both signed-in sidebars, sign-in and the 404 page.
#[test]
fn pages_without_the_public_footer_carry_the_legal_links() {
    assert_eq!(APP_LAYOUT.matches("<LegalLinks/>").count(), 2);
    assert!(LOGIN.contains("<LegalLinks/>"));
    assert!(NOT_FOUND.contains("<LegalLinks/>"));
}

#[test]
fn the_landing_page_renders_inside_the_public_layout() {
    assert!(LANDING.contains("<PublicLayout>"));
}

#[test]
fn both_pages_render_inside_the_public_layout() {
    assert!(LEGAL_LAYOUT.contains("<PublicLayout>"));
    assert!(PRIVACY.contains("<LegalDocument"));
    assert!(TERMS.contains("<LegalDocument"));
}

/// A session check or server function would gate the page or push its text
/// out of the first response that reviewers and crawlers fetch.
#[test]
fn neither_page_needs_a_session_or_a_server_function() {
    for source in [PRIVACY, TERMS, LEGAL_LAYOUT] {
        assert!(!source.contains("crate::server"));
        assert!(!source.contains("Resource::"));
        assert!(!source.contains("AppShell"));
    }
}

#[test]
fn both_pages_set_a_title_and_a_last_updated_date() {
    for source in [PRIVACY, TERMS] {
        assert!(source.contains("<Title text="));
        assert!(source.contains("updated=UPDATED"));
    }
    assert!(LEGAL_LAYOUT.contains(r#""Last updated: "{updated}"#));
}

#[test]
fn the_privacy_policy_carries_the_youtube_disclosures() {
    for required in [
        "Zayden uses YouTube API Services.",
        "channels.list?mine=true",
        "https://security.google.com/settings/security/permissions",
        "https://policies.google.com/privacy",
        "https://developers.google.com/terms/api-services-user-data-policy",
        "\"Zayden's use and transfer of information received from Google APIs will \"",
        "\", including the Limited Use requirements.\"",
    ] {
        assert!(PRIVACY.contains(required), "privacy policy is missing {required}");
    }
}

#[test]
fn the_terms_bind_youtube_users_to_the_youtube_terms() {
    for required in [
        "\"By using the YouTube features, you agree to be bound by the \"",
        "https://www.youtube.com/t/terms",
        "https://policies.google.com/privacy",
        "https://discord.com/terms",
    ] {
        assert!(TERMS.contains(required), "terms are missing {required}");
    }
}

/// The purge job in `bot/src/guild_retention.rs` deletes on this window; the
/// policy must quote the same number.
#[cfg(feature = "ssr")]
#[test]
fn the_privacy_policy_states_the_guild_retention_window() {
    let window = format!("kept for {} days", zayden_app::guilds::RETENTION_DAYS);
    assert!(PRIVACY.contains(&window), "privacy policy is missing {window:?}");
}
