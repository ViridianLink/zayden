//! Parsing of the campaign-posts endpoint, against payloads shaped like the
//! ones Patreon returns (captured 2026-09-04).
//!
//! Two behaviours here are load-bearing and easy to regress: a post missing a
//! field the announcement needs is skipped rather than failing its page, and
//! the cursor is `None` on the last page so the caller keeps the cursor that
//! reached it.

use std::fs;

use patreon::api::parse_posts_page;
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
    assert_eq!(post.url, "https://www.patreon.com/posts/august-devlog-1001");
    assert!(post.is_public);
    assert_eq!(post.published_at.to_string(), "2026-08-01T12:00:00Z");
    assert!(
        post.content_html.as_deref().is_some_and(|c| c.contains("<strong>")),
        "{:?}",
        post.content_html
    );
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
