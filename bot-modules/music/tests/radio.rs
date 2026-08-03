//! Radio config validation, the genre catalogue, track shaping, the reconnect and
//! failover budget, and the interaction between radio mode and the queue's loop
//! handling.
//!
//! The behaviours pinned here are the ones that are invisible until they break
//! in production:
//!
//! * A malformed `config.toml` entry must be dropped, not panic the bot at boot.
//! * The genre catalogue is the Discord choice list, so it has hard shape
//!   constraints.
//! * A station must never re-enter the queue or the history. It is an endless live
//!   stream, so `LoopMode::Queue` would otherwise cycle it forever and the real
//!   queue would never play again.
//! * A dead station must stop being retried. Reconnecting unconditionally on
//!   `TrackEvent::End` is a hot loop against someone else's server.
//! * The raw `stream_url` must never reach a user-visible field.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use music::{
    AdvanceAction,
    Genre,
    LoopMode,
    RadioSession,
    RadioStation,
    TrackSource,
    advance_action,
    next_retry_count,
    records_history,
    should_reconnect,
    station_track,
};
use serenity::all::UserId;
use zayden_app::config::radio;

fn station(id: &str, name: &str, genre: &str) -> RadioStation {
    RadioStation {
        id: id.to_string(),
        name: name.to_string(),
        stream_url: format!("https://example.test/{id}.mp3"),
        genre: genre.to_string(),
        homepage: Some(format!("https://example.test/{id}")),
        logo_url: None,
    }
}

fn session(genre: Genre, ids: &[&str]) -> Option<RadioSession> {
    let pool: Vec<Arc<RadioStation>> =
        ids.iter().map(|id| Arc::new(station(id, id, genre.value()))).collect();

    RadioSession::new(genre, pool)
}

// ── config validation ────────────────────────────────────────────────────────

#[test]
fn validate_all_keeps_well_formed_stations() {
    let stations = radio::validate_all(vec![
        station("lofi", "Lofi Hip Hop", "lofi"),
        station("jazz", "Jazz FM", "jazz"),
    ]);

    assert_eq!(stations.len(), 2);
}

#[test]
fn validate_all_drops_entries_with_empty_required_fields() {
    let mut blank_id = station("x", "Has No Id", "pop");
    blank_id.id = String::new();
    let mut blank_name = station("y", "", "pop");
    blank_name.name = "   ".to_string();

    let stations = radio::validate_all(vec![
        blank_id,
        blank_name,
        station("ok", "Fine", "pop"),
    ]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.id.as_str()), Some("ok"));
}

#[test]
fn validate_all_drops_non_http_stream_urls() {
    // A `file://` or bare-host entry would make songbird fail at play time with
    // an opaque error; reject it at boot instead.
    let mut local = station("local", "Local File", "pop");
    local.stream_url = "file:///etc/passwd".to_string();
    let mut bare = station("bare", "No Scheme", "pop");
    bare.stream_url = "example.test/stream".to_string();

    let stations =
        radio::validate_all(vec![local, bare, station("ok", "Fine", "pop")]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.id.as_str()), Some("ok"));
}

#[test]
fn validate_all_drops_unrecognised_genre_tags() {
    // A typo'd tag would silently make the station unreachable — no choice in the
    // picker maps to it — so drop it loudly at boot instead.
    let stations = radio::validate_all(vec![
        station("typo", "Typo'd Tag", "poop"),
        station("empty", "No Tag", ""),
        station("ok", "Fine", "pop"),
    ]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.id.as_str()), Some("ok"));
}

#[test]
fn validate_all_drops_duplicate_ids_keeping_the_first() {
    // `id` is the track's `source_id`, so a duplicate would make station lookup
    // ambiguous at stream time.
    let mut second = station("lofi", "Impostor", "lofi");
    second.stream_url = "https://example.test/other.mp3".to_string();

    let stations =
        radio::validate_all(vec![station("lofi", "Lofi Hip Hop", "lofi"), second]);

    assert_eq!(stations.len(), 1);
    assert_eq!(stations.first().map(|s| s.name.as_str()), Some("Lofi Hip Hop"));
}

#[test]
fn a_station_with_no_homepage_still_has_a_display_url() {
    let mut no_homepage = station("lofi", "Lofi", "lofi");
    no_homepage.homepage = None;

    assert_eq!(no_homepage.display_url(), no_homepage.stream_url);
}

// ── the genre catalogue ──────────────────────────────────────────────────────

#[test]
fn the_catalogue_fits_discords_choice_cap() {
    // `Genre::ALL` is rendered verbatim as the choices on `/music radio play`, and
    // Discord rejects a command option with more than 25 of them at `set_commands`
    // time — in every guild, at runtime, not at compile time.
    assert!(
        Genre::ALL.len() <= 25,
        "the catalogue has {} entries; Discord's cap is 25",
        Genre::ALL.len(),
    );
}

#[test]
fn every_genre_round_trips_through_its_value_and_its_label() {
    // Discord sends the value; `config.toml` is hand-written and may use either.
    for genre in Genre::ALL {
        assert_eq!(Genre::from_value(genre.value()), Some(genre));
        assert_eq!(Genre::from_value(genre.label()), Some(genre));
        assert_eq!(Genre::from_value(&genre.value().to_uppercase()), Some(genre));
        assert_eq!(
            Genre::from_value(&format!("  {}  ", genre.value())),
            Some(genre)
        );
    }

    assert_eq!(Genre::from_value("not-a-genre"), None);
    assert_eq!(Genre::from_value(""), None);
}

#[test]
fn genre_values_are_unique_stable_identifiers() {
    // Values are the `config.toml` tags and the Discord choice values, so a
    // collision would make two catalogue entries indistinguishable.
    let values: HashSet<&str> = Genre::ALL.iter().map(|g| g.value()).collect();
    assert_eq!(values.len(), Genre::ALL.len(), "duplicate genre value");

    let labels: HashSet<&str> = Genre::ALL.iter().map(|g| g.label()).collect();
    assert_eq!(labels.len(), Genre::ALL.len(), "duplicate genre label");

    for genre in Genre::ALL {
        let value = genre.value();
        assert!(!value.is_empty(), "{value:?} is empty");
        assert!(
            value
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{value:?} must be lowercase kebab-case to stay stable in config.toml",
        );
    }
}

// ── genre pools ──────────────────────────────────────────────────────────────

#[test]
fn a_pool_holds_every_station_tagged_with_that_genre() {
    let stations = radio::validate_all(vec![
        station("a", "A", "rock"),
        station("b", "B", "pop"),
        station("c", "C", "Rock"),
    ]);

    let rock = radio::pool(&stations, Genre::Rock);
    assert_eq!(rock.len(), 2, "the tag is matched case-insensitively");
    assert_eq!(radio::pool(&stations, Genre::Pop).len(), 1);
    assert!(radio::pool(&stations, Genre::Jazz).is_empty());
}

#[test]
fn unbacked_reports_exactly_the_genres_with_no_stations() {
    let stations = radio::validate_all(vec![
        station("a", "A", "rock"),
        station("b", "B", "pop"),
    ]);

    let unbacked: HashSet<Genre> = radio::unbacked(&stations).into_iter().collect();

    assert!(!unbacked.contains(&Genre::Rock));
    assert!(!unbacked.contains(&Genre::Pop));
    assert!(unbacked.contains(&Genre::Jazz));
    assert_eq!(unbacked.len(), Genre::ALL.len() - 2);

    assert_eq!(radio::unbacked(&[]).len(), Genre::ALL.len());
}

// ── failover ─────────────────────────────────────────────────────────────────

#[test]
fn a_session_needs_at_least_one_station() {
    assert!(RadioSession::new(Genre::Pop, Vec::new()).is_none());
}

#[test]
fn failover_walks_the_whole_pool_without_repeating() {
    // The user picked a genre, not a station, so one dead stream must not end radio
    // mode while other stations for that genre are still untried.
    let Some(mut session) = session(Genre::Rock, &["a", "b", "c"]) else {
        panic!("a non-empty pool must yield a session");
    };

    let mut seen = vec![session.station.id.clone()];
    while let Some(station) = session.failover() {
        assert_eq!(
            station.id, session.station.id,
            "the session must track the swap"
        );
        seen.push(station.id.clone());
        assert!(seen.len() <= 3, "failover handed out a station twice");
    }

    seen.sort();
    assert_eq!(seen, ["a", "b", "c"]);
}

#[test]
fn failover_gives_up_once_the_pool_is_exhausted() {
    // `None` is what makes the end-of-track handler leave radio mode and resume the
    // queue; without it a fully-dead genre would spin.
    let Some(mut single) = session(Genre::Pop, &["only"]) else {
        panic!("a non-empty pool must yield a session");
    };
    assert!(single.failover().is_none());
    assert!(single.failover().is_none(), "exhaustion must be sticky");
}

// ── track shaping ────────────────────────────────────────────────────────────

#[test]
fn station_track_is_a_live_radio_track_with_no_duration() {
    let station = station("lofi", "Lofi Hip Hop", "lofi");
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
    let station = station("lofi", "Lofi Hip Hop", "lofi");
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
