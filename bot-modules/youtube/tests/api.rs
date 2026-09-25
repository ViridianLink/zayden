//! Parsing of the `playlistItems` and `channels?mine=true` responses.
//!
//! Load-bearing: anything but a public upload is dropped, since a scheduled
//! video is announced by the first poll after it goes public, not before.

use std::fs;

use serde_json::{Value, json};
use youtube::YoutubeError;
use youtube::api::{parse_own_channel, parse_uploads};

fn load(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));

    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

#[test]
fn only_public_uploads_are_kept_newest_first() {
    let ids: Vec<String> = parse_uploads(&load("playlist_items.json"), "UCexample")
        .into_iter()
        .map(|video| video.id)
        .collect();

    assert_eq!(ids, ["vid-new", "vid-fallback"]);
}

#[test]
fn a_video_carries_its_channel_title_and_publish_time() {
    let videos = parse_uploads(&load("playlist_items.json"), "UCexample");

    assert_eq!(videos[0].channel_id, "UCexample");
    assert_eq!(videos[0].title, "Brand new upload");
    assert_eq!(videos[0].published_at, "2026-09-20T18:00:00Z".parse().unwrap());
}

/// `videoPublishedAt` is the video's own date; the playlist's `publishedAt` is
/// only a fallback for items that omit it.
#[test]
fn a_missing_video_date_falls_back_to_the_playlist_date() {
    let videos = parse_uploads(&load("playlist_items.json"), "UCexample");

    assert_eq!(videos[1].published_at, "2026-09-17T09:30:00Z".parse().unwrap());
}

#[test]
fn an_empty_or_unexpected_body_yields_no_videos() {
    assert_eq!(parse_uploads(&json!({}), "UCexample"), []);
    assert_eq!(parse_uploads(&json!({ "items": "nope" }), "UCexample"), []);
}

#[test]
fn the_authorised_channel_and_its_uploads_playlist_are_read() {
    let channel = parse_own_channel(&load("channels_mine.json")).unwrap();

    assert_eq!(channel.id, "UCexample");
    assert_eq!(channel.title, "Example Creator");
    assert_eq!(channel.uploads_playlist_id, "UUexample");
}

#[test]
fn an_account_without_a_channel_is_reported_as_such() {
    let result = parse_own_channel(&json!({ "items": [] }));

    assert!(matches!(result, Err(YoutubeError::NoChannel)), "{result:?}");
}
