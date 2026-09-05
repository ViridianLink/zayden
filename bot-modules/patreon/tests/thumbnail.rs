//! The Patreon API exposes no image for a post, so `og:image` on the public
//! post page is the only source. Every failure has to be survivable: the
//! announcement goes out without a thumbnail rather than not at all.

use std::fs;

use patreon::thumbnail::og_image;

fn load(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.html", env!("CARGO_MANIFEST_DIR"));

    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn og_image_is_extracted_from_a_post_page() {
    let url = og_image(&load("patreon_post"));

    assert_eq!(
        url.as_deref(),
        Some(
            "https://c10.patreonusercontent.com/4/patreon-media/p/post/1001/hero.jpg?token=abc"
        )
    );
}

#[test]
fn a_page_without_an_og_image_yields_none() {
    assert_eq!(og_image(&load("patreon_post_no_image")), None);
}

/// A Cloudflare interstitial or an error page is still HTML; it just has no
/// `og:image`.
#[test]
fn a_challenge_page_yields_none() {
    assert_eq!(og_image("<html><body>Just a moment...</body></html>"), None);
}

#[test]
fn an_empty_content_attribute_is_not_a_url() {
    assert_eq!(og_image(r#"<meta property="og:image" content="  ">"#), None);
}

#[test]
fn a_non_html_body_does_not_panic() {
    assert_eq!(og_image("{\"error\": \"nope\"}"), None);
}
