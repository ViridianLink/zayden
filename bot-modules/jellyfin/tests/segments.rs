//! Segment coverage. Intro Skipper has not analysed every episode, so the
//! reported numbers must never imply coverage that does not exist.

use jellyfin::transport::segments::{MediaSegment, SegmentStats};

fn segment(kind: &str, seconds: i64) -> MediaSegment {
    MediaSegment {
        segment_type: kind.to_owned(),
        start_ticks: 0,
        end_ticks: seconds * 10_000_000,
    }
}

#[test]
fn skippable_kinds_are_recognised() {
    for kind in ["Intro", "Outro", "Recap", "Preview", "Commercial"] {
        assert!(segment(kind, 30).is_skippable(), "`{kind}` should be skippable");
    }
}

#[test]
fn unknown_segments_are_not_subtracted() {
    assert!(!segment("Unknown", 30).is_skippable());
    assert!(!segment("", 30).is_skippable());
}

#[test]
fn seconds_come_from_the_tick_span() {
    assert_eq!(segment("Intro", 90).seconds(), 90);
}

#[test]
fn a_negative_span_cannot_produce_negative_time() {
    let inverted = MediaSegment {
        segment_type: "Intro".to_owned(),
        start_ticks: 100,
        end_ticks: 0,
    };

    assert_eq!(inverted.seconds(), 0);
}

#[test]
fn coverage_is_reported_honestly() {
    let stats = SegmentStats { episodes: 10, measured: 1, skippable_seconds: 180 };

    assert!(stats.has_coverage());
    assert_eq!(stats.coverage_percent(), 10);
}

#[test]
fn no_data_is_zero_coverage_not_a_division_by_zero() {
    let stats = SegmentStats::default();

    assert!(!stats.has_coverage());
    assert_eq!(stats.coverage_percent(), 0);
}
