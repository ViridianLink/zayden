//! The Patreon v2 API exposes no image for a post, so the public post
//! endpoint the web app uses is the source. Every failure has to be
//! survivable: the announcement goes out without a thumbnail rather than not
//! at all.

use std::fs;

use patreon::thumbnail::{api_image, content_image};

fn load(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));

    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn the_large_image_is_extracted_from_a_post() {
    let url = api_image(&load("patreon_post_api"));

    assert_eq!(
        url.as_deref(),
        Some(
            "https://c10.patreonusercontent.com/4/patreon-media/p/post/1001/1080/1.png?token-hash=abc"
        )
    );
}

#[test]
fn the_default_image_is_used_without_a_large_one() {
    let json = r#"{"data":{"attributes":{"image":{"url":"https://c10.patreonusercontent.com/a.png"}}}}"#;

    assert_eq!(
        api_image(json).as_deref(),
        Some("https://c10.patreonusercontent.com/a.png")
    );
}

#[test]
fn a_post_without_an_image_yields_none() {
    assert_eq!(api_image(&load("patreon_post_api_no_image")), None);
}

/// A Cloudflare interstitial or an error page is HTML, not JSON.
#[test]
fn a_challenge_page_yields_none() {
    assert_eq!(api_image("<html><body>Just a moment...</body></html>"), None);
}

#[test]
fn an_empty_url_is_not_a_url() {
    let json = r#"{"data":{"attributes":{"image":{"url":"  ","large_url":""}}}}"#;

    assert_eq!(api_image(json), None);
}

#[test]
fn an_error_body_yields_none() {
    assert_eq!(api_image(r#"{"errors":[{"status":"404"}]}"#), None);
}

#[test]
fn the_first_https_image_in_the_body_is_the_fallback() {
    let html = r#"<p>Intro</p><img src="data:image/png;base64,AA"><img src="https://c10.patreonusercontent.com/a.png?token-hash=x&amp;token-time=1"><img src="https://c10.patreonusercontent.com/b.png">"#;

    assert_eq!(
        content_image(html).as_deref(),
        Some("https://c10.patreonusercontent.com/a.png?token-hash=x&token-time=1")
    );
}

#[test]
fn a_body_without_images_yields_none() {
    assert_eq!(content_image("<p>No pictures here.</p>"), None);
}
