//! Radio station config validation, track shaping, reconnect budget and the
//! interaction between radio mode and the queue's loop handling.
//!
//! The behaviours pinned here are the ones that are invisible until they break
//! in production:
//!
//! * A malformed `config.toml` entry must be dropped, not panic the bot at boot.
//! * A station must never re-enter the queue or the history. It is an endless live
//!   stream, so `LoopMode::Queue` would otherwise cycle it forever and the real
//!   queue would never play again.
//! * A dead station must stop being retried. Reconnecting unconditionally on
//!   `TrackEvent::End` is a hot loop against someone else's server.
//! * The raw `stream_url` must never reach a user-visible field.

use std::time::Duration;

use music::{
    AdvanceAction,
    LoopMode,
    RadioStation,
    TrackSource,
    advance_action,
    next_retry_count,
    records_history,
    should_reconnect,
    station_track,
};
use serenity::all::UserId;

fn station(id: &str, name: &str, genre: Option<&str>) -> RadioStation {
    RadioStation {
        id: id.to_string(),
        name: name.to_string(),
        stream_url: format!("https://example.test/{id}.mp3"),
        genre: genre.map(ToOwned::to_owned),
        homepage: Some(format!("https://example.test/{id}")),
        logo_url: None,
    }
}

// ── config validation ────────────────────────────────────────────────────────

#[test]
fn validate_all_keeps_well_formed_stations() {
    let stations = zayden_app::config::radio::validate_all(vec![
        station("lofi", "Lofi Hip Hop", Some("Chill")),
        station("jazz", "Jazz FM", Some("Jazz")),
    ]);

    assert_eq!(stations.len(), 2);
}

#[test]
fn validate_all_drops_entries_with_empty_required_fields() {
    let mut blank_id = station("x", "Has No Id", None);
    blank_id.id = String::new();
    let mut blank_name = station("y", "", None);
    blank_name.name = "   ".to_string();

    let stations = zayden_app::config::radio::validate_all(vec![
        blank_id,
        blank_name,
        station("ok", "Fine", None),
    ]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.id.as_str()), Some("ok"));
}

#[test]
fn validate_all_drops_non_http_stream_urls() {
    // A `file://` or bare-host entry would make songbird fail at play time with
    // an opaque error; reject it at boot instead.
    let mut local = station("local", "Local File", None);
    local.stream_url = "file:///etc/passwd".to_string();
    let mut bare = station("bare", "No Scheme", None);
    bare.stream_url = "example.test/stream".to_string();

    let stations = zayden_app::config::radio::validate_all(vec![
        local,
        bare,
        station("ok", "Fine", None),
    ]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.id.as_str()), Some("ok"));
}

#[test]
fn validate_all_drops_duplicate_ids_keeping_the_first() {
    // `id` is the autocomplete value and the track's `source_id`, so a duplicate
    // would make station lookup ambiguous at stream time.
    let mut second = station("lofi", "Impostor", None);
    second.stream_url = "https://example.test/other.mp3".to_string();

    let stations = zayden_app::config::radio::validate_all(vec![
        station("lofi", "Lofi Hip Hop", Some("Chill")),
        second,
    ]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.name.as_str()), Some("Lofi Hip Hop"));
}

#[test]
fn a_station_with_no_homepage_still_has_a_display_url() {
    let mut no_homepage = station("lofi", "Lofi", None);
    no_homepage.homepage = None;

    assert_eq!(no_homepage.display_url(), no_homepage.stream_url);
}

// ── track shaping ────────────────────────────────────────────────────────────

#[test]
fn station_track_is_a_live_radio_track_with_no_duration() {
    let station = station("lofi", "Lofi Hip Hop", Some("Chill"));
    let track = station_track(&station, UserId::new(7));

    assert_eq!(track.source, TrackSource::Radio);
    assert_eq!(track.source_id, "lofi");
    assert_eq!(track.title, "Lofi Hip Hop");
    assert_eq!(track.requested_by, UserId::new(7));
    // `is_live` is what makes `/music seek` reject the stream, and a `None`
    // duration is what makes the progress bar render the live marker.
    assert!(track.is_live);
    assert_eq!(track.duration, None);
}

#[test]
fn station_track_never_exposes_the_raw_stream_url() {
    let station = station("lofi", "Lofi Hip Hop", None);
    let track = station_track(&station, UserId::new(1));

    assert_eq!(track.url, "https://example.test/lofi");
    assert_ne!(
        track.url, station.stream_url,
        "the stream URL is an implementation detail and must stay out of embeds"
    );
}

// ── queue interaction ────────────────────────────────────────────────────────

#[test]
fn radio_is_dropped_rather_than_replayed_or_requeued() {
    for loop_mode in [LoopMode::Off, LoopMode::Track, LoopMode::Queue] {
        assert_eq!(
            advance_action(loop_mode, true),
            AdvanceAction::Drop,
            "a radio station must never re-enter the queue under {loop_mode:?}",
        );
    }
}

#[test]
fn normal_tracks_keep_their_loop_behaviour() {
    assert_eq!(advance_action(LoopMode::Off, false), AdvanceAction::Drop);
    assert_eq!(advance_action(LoopMode::Track, false), AdvanceAction::Replay);
    assert_eq!(advance_action(LoopMode::Queue, false), AdvanceAction::Requeue);
}

#[test]
fn radio_never_enters_the_play_history() {
    assert!(!records_history(TrackSource::Radio));
    assert!(records_history(TrackSource::YouTube));
    assert!(records_history(TrackSource::Spotify));
}

// ── reconnect budget ─────────────────────────────────────────────────────────

#[test]
fn a_transient_drop_after_a_healthy_stretch_always_reconnects() {
    let played = Duration::from_secs(3_600);

    assert!(should_reconnect(played, 0));
    // Even at the retry ceiling: the budget is about *consecutive fast*
    // failures, so a station that ran for an hour has earned a fresh one.
    assert!(should_reconnect(played, 250));
    assert_eq!(next_retry_count(played, 250), 0);
}

#[test]
fn instant_failures_are_retried_a_bounded_number_of_times() {
    let played = Duration::from_secs(1);

    let mut retries = 0;
    let mut attempts = 0;
    while should_reconnect(played, retries) {
        retries = next_retry_count(played, retries);
        attempts += 1;
        assert!(attempts <= 10, "reconnect budget failed to terminate");
    }

    assert_eq!(attempts, 3, "a dead station must stop being retried");
}

#[test]
fn the_retry_counter_cannot_overflow() {
    // `next_retry_count` takes a u8; a station flapping for a long time must
    // saturate rather than wrap back into the "keep trying" range.
    assert_eq!(next_retry_count(Duration::from_secs(0), u8::MAX), u8::MAX);
    assert!(!should_reconnect(Duration::from_secs(0), u8::MAX));
}

// ── autocomplete filtering ───────────────────────────────────────────────────

#[test]
fn an_empty_query_lists_stations() {
    let stations = vec![
        station("lofi", "Lofi Hip Hop", Some("Chill")),
        station("a", "B", None),
    ];

    assert_eq!(music::autocomplete::matching_stations(&stations, "").len(), 2);
}

#[test]
fn filtering_is_case_insensitive_and_matches_name_genre_and_id() {
    let stations = vec![
        station("lofi", "Lofi Hip Hop", Some("Chill")),
        station("defcon", "DEF CON Radio", Some("Electronic")),
    ];

    let by_name = music::autocomplete::matching_stations(&stations, "HIP");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name.first().map(|s| s.id.as_str()), Some("lofi"));

    let by_genre = music::autocomplete::matching_stations(&stations, "electronic");
    assert_eq!(by_genre.first().map(|s| s.id.as_str()), Some("defcon"));

    let by_id = music::autocomplete::matching_stations(&stations, "DEFC");
    assert_eq!(by_id.first().map(|s| s.id.as_str()), Some("defcon"));
}

#[test]
fn autocomplete_never_exceeds_discords_choice_limit() {
    // Discord rejects an autocomplete response with more than 25 choices, and
    // the station list is operator-supplied, so it can grow past that freely.
    let stations: Vec<RadioStation> = (0..60)
        .map(|i| station(&format!("s{i}"), &format!("Station {i}"), None))
        .collect();

    assert_eq!(music::autocomplete::matching_stations(&stations, "").len(), 25);
    assert_eq!(
        music::autocomplete::matching_stations(&stations, "Station").len(),
        25
    );
}
