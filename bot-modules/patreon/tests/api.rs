//! Parsing of the campaign-posts endpoint, against payloads shaped like the
//! ones Patreon returns (captured 2026-09-04).
//!
//! Two behaviours here are load-bearing and easy to regress: a post missing a
//! field the announcement needs is skipped rather than failing its page, and
//! the cursor is `None` on the last page so the caller keeps the cursor that
//! reached it.

use std::fs;

use patreon::api::{parse_campaign, parse_posts_page, truncate_log};
use patreon::error::PatreonError;
use serde_json::Value;

const CAMPAIGN: &str = "555000";

fn load(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));

    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

#[test]
fn a_page_yields_its_posts_in_order() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);

    let ids: Vec<&str> = page.posts.iter().map(|post| post.id.as_str()).collect();
    assert_eq!(ids, ["1001", "1002"]);
}

/// The listing endpoint does not include a campaign relationship, so the
/// campaign the request was made against has to fill it in.
#[test]
fn posts_inherit_the_requested_campaign() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);

    assert!(page.posts.iter().all(|post| post.campaign_id == CAMPAIGN));
}

#[test]
fn attributes_are_mapped() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);
    let post = page.posts.first().expect("the fixture has a first post");

    assert_eq!(post.title.as_deref(), Some("August devlog"));
    assert_eq!(post.url, "https://www.patreon.com/creator/posts/august-devlog-1001");
    assert!(post.is_public);
    assert_eq!(post.published_at.to_string(), "2026-08-01T12:00:00Z");
    assert!(
        post.content_html.as_deref().is_some_and(|c| c.contains("<strong>")),
        "{:?}",
        post.content_html
    );
}

/// The live API returns a site-relative path; an already-absolute URL is kept.
#[test]
fn post_urls_are_made_absolute() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);

    let urls: Vec<&str> = page.posts.iter().map(|post| post.url.as_str()).collect();
    assert_eq!(urls, [
        "https://www.patreon.com/creator/posts/august-devlog-1001",
        "https://www.patreon.com/posts/1002",
    ]);
}

#[test]
fn a_null_title_and_body_survive_as_none() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);
    let post = page.posts.get(1).expect("the fixture has a second post");

    assert_eq!(post.title, None);
    assert_eq!(post.content_html, None);
    assert!(!post.is_public);
}

/// Post 1003 in the fixture has no `url`. Announcing it is impossible, but one
/// malformed entry must not cost us the rest of the page.
#[test]
fn a_post_missing_a_required_field_is_skipped_not_fatal() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);

    assert_eq!(page.posts.len(), 2);
    assert!(page.posts.iter().all(|post| post.id != "1003"));
}

#[test]
fn the_next_cursor_is_read_from_pagination_meta() {
    let page = parse_posts_page(&load("patreon_posts_page"), CAMPAIGN);

    assert_eq!(page.next_cursor.as_deref(), Some("cursor-page-2"));
}

/// A null `next` ends the walk. The caller keeps the cursor that produced this
/// page, so the following poll resumes from a page that still exists.
#[test]
fn the_last_page_has_no_next_cursor() {
    let page = parse_posts_page(&load("patreon_posts_last_page"), CAMPAIGN);

    assert_eq!(page.next_cursor, None);
    assert_eq!(page.posts.len(), 1);
}

#[test]
fn an_empty_or_unexpected_body_yields_an_empty_page() {
    let page = parse_posts_page(&Value::Null, CAMPAIGN);

    assert_eq!(page.posts, []);
    assert_eq!(page.next_cursor, None);
}

/// `creation_name` is the campaign's tagline, so the name comes from the
/// included creator.
#[test]
fn the_campaign_is_named_after_its_creator() {
    let (id, name) = parse_campaign(&load("patreon_campaign")).unwrap();

    assert_eq!(id, "555000");
    assert_eq!(name.as_deref(), Some("Creator Name"));
}

#[test]
fn a_campaign_without_its_creator_falls_back_to_the_vanity() {
    let body = serde_json::json!({
        "data": [{ "id": "555000", "type": "campaign", "attributes": { "vanity": "creatorvanity" } }]
    });

    let (_, name) = parse_campaign(&body).unwrap();

    assert_eq!(name.as_deref(), Some("creatorvanity"));
}

#[test]
fn an_account_without_a_campaign_is_rejected() {
    let body = serde_json::json!({ "data": [] });

    assert!(matches!(parse_campaign(&body), Err(PatreonError::NoCampaign)));
}

#[test]
fn a_long_log_body_is_cut_on_a_char_boundary() {
    let body = "\u{e9}".repeat(600);

    assert_eq!(truncate_log(&body).chars().count(), 500);
    assert_eq!(truncate_log("short"), "short");
}
