//! The webhook is an unauthenticated public endpoint, so the signature check is
//! the only thing standing between anyone on the internet and an announcement
//! in every subscribed guild. These pin it against an independent vector rather
//! than against our own implementation.

use std::fs;

use patreon::webhook::{parse_post, verify};

const SECRET: &str = "test-webhook-secret";

/// HMAC-MD5 of `patreon_webhook_publish.json` under `SECRET`, computed with
/// Python's `hmac`/`hashlib` rather than this crate.
const FIXTURE_SIGNATURE: &str = "7558c8e72e896eadb8c94e026c3d3e77";

fn payload() -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/patreon_webhook_publish.json",
        env!("CARGO_MANIFEST_DIR")
    );

    fs::read(&path).unwrap_or_default()
}

/// RFC 2202, test case 2: an HMAC-MD5 vector that predates this codebase.
#[test]
fn the_rfc_2202_vector_verifies() {
    assert!(verify(
        b"what do ya want for nothing?",
        "750c783e6ab0b503eaa86e310a5db738",
        "Jefe"
    ));
}

#[test]
fn a_correctly_signed_payload_verifies() {
    assert!(verify(&payload(), FIXTURE_SIGNATURE, SECRET));
}

/// Patreon sends lowercase, but hex case carries no meaning and rejecting the
/// uppercase form would only break on a header we do not control.
#[test]
fn digest_case_does_not_matter() {
    assert!(verify(&payload(), &FIXTURE_SIGNATURE.to_uppercase(), SECRET));
}

#[test]
fn a_tampered_body_is_rejected() {
    let mut body = payload();
    body.push(b' ');

    assert!(!verify(&body, FIXTURE_SIGNATURE, SECRET));
}

#[test]
fn the_wrong_secret_is_rejected() {
    assert!(!verify(&payload(), FIXTURE_SIGNATURE, "not-the-secret"));
}

#[test]
fn a_missing_or_malformed_signature_is_rejected() {
    assert!(!verify(&payload(), "", SECRET));
    assert!(!verify(&payload(), "not hex", SECRET));
    assert!(!verify(&payload(), "abc", SECRET), "odd-length hex");
    let short: String = FIXTURE_SIGNATURE.chars().take(30).collect();
    assert!(!verify(&payload(), &short, SECRET), "short digest");
}

/// Unlike the listing endpoint, the webhook resource carries its own campaign,
/// which is what decides where the post is announced.
#[test]
fn the_post_is_parsed_with_its_campaign() {
    let post =
        parse_post(&payload()).expect("the fixture is a valid publish payload");

    assert_eq!(post.id, "2001");
    assert_eq!(post.campaign_id, "555000");
    assert_eq!(post.title.as_deref(), Some("Webhook post"));
    assert_eq!(post.url, "https://www.patreon.com/posts/webhook-post-2001");
    assert!(!post.is_public);
    assert_eq!(post.published_at.to_string(), "2026-09-02T18:45:00Z");
}

#[test]
fn a_payload_without_a_campaign_is_rejected() {
    let body = br#"{"data":{"id":"1","attributes":{"url":"https://x","published_at":"2026-01-01T00:00:00Z"}}}"#;

    assert!(parse_post(body).is_err());
}

#[test]
fn a_payload_that_is_not_json_is_rejected() {
    assert!(parse_post(b"<html>nope</html>").is_err());
}
