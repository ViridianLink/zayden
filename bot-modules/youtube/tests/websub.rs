use youtube::websub::{Mode, callback_url, channel_from_topic, topic_url, verify};

/// RFC 2202 test case 2 for HMAC-SHA1.
const RFC_KEY: &str = "Jefe";
const RFC_DATA: &[u8] = b"what do ya want for nothing?";
const RFC_DIGEST: &str = "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79";

#[test]
fn the_rfc_2202_vector_verifies() {
    assert!(verify(RFC_DATA, &format!("sha1={RFC_DIGEST}"), RFC_KEY));
}

#[test]
fn digest_case_does_not_matter() {
    let upper = RFC_DIGEST.to_uppercase();
    assert!(verify(RFC_DATA, &format!("sha1={upper}"), RFC_KEY));
}

#[test]
fn a_tampered_body_is_rejected() {
    assert!(!verify(
        b"what do ya want for something?",
        &format!("sha1={RFC_DIGEST}"),
        RFC_KEY
    ));
}

#[test]
fn the_wrong_secret_is_rejected() {
    assert!(!verify(RFC_DATA, &format!("sha1={RFC_DIGEST}"), "not-jefe"));
}

#[test]
fn a_missing_or_malformed_signature_is_rejected() {
    assert!(!verify(RFC_DATA, "", RFC_KEY));
    assert!(!verify(RFC_DATA, RFC_DIGEST, RFC_KEY));
    assert!(!verify(RFC_DATA, &format!("sha256={RFC_DIGEST}"), RFC_KEY));
    assert!(!verify(RFC_DATA, "sha1=zz", RFC_KEY));
    assert!(!verify(RFC_DATA, "sha1=abc", RFC_KEY));
}

#[test]
fn a_topic_round_trips_to_its_channel() {
    assert_eq!(
        channel_from_topic(&topic_url("UCexample")).as_deref(),
        Some("UCexample")
    );
}

/// The verification callback trusts the topic only to name a channel, so a
/// look-alike feed on another host must not.
#[test]
fn a_foreign_topic_names_no_channel() {
    assert_eq!(
        channel_from_topic(
            "https://evil.test/xml/feeds/videos.xml?channel_id=UCexample"
        ),
        None
    );
    assert_eq!(channel_from_topic("not a url"), None);
    assert_eq!(
        channel_from_topic(
            "https://www.youtube.com/xml/feeds/videos.xml?playlist_id=PL1"
        ),
        None
    );
}

#[test]
fn the_callback_names_its_channel() {
    assert_eq!(
        callback_url("https://dash.test/webhooks/youtube", "UCexample").unwrap(),
        "https://dash.test/webhooks/youtube?channel=UCexample"
    );
}

#[test]
fn modes_round_trip() {
    for mode in [Mode::Subscribe, Mode::Unsubscribe] {
        assert_eq!(Mode::from_param(mode.as_str()), Some(mode));
    }
    assert_eq!(Mode::from_param("denied"), None);
}
