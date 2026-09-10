//! Binge arithmetic, against a hand-computed fixture.

use jellyfin::transport::segments::SegmentStats;
use watch::discovery::binge::{BingePlan, format_duration};

/// Reacher season 1: eight episodes, runtimes read off the server.
///
/// 54.1 + 52.9 + 47.5 + 46.3 + 48.8 + 48.7 + 41.7 + 52.5 = 392.5 minutes,
/// which is 23,550 seconds (6h 32m).
fn reacher() -> BingePlan {
    let minutes = [54.1_f64, 52.9, 47.5, 46.3, 48.8, 48.7, 41.7, 52.5];
    #[expect(
        clippy::cast_possible_truncation,
        reason = "runtimes in minutes are small integers once scaled to seconds"
    )]
    let total_seconds = minutes.iter().map(|m| (m * 60.0) as i64).sum();

    BingePlan {
        episodes: 8,
        total_seconds,
        // Near-complete coverage: recap + outro on all but two episodes.
        segments: SegmentStats {
            episodes: 8,
            measured: 6,
            skippable_seconds: 1_026,
        },
    }
}

#[test]
fn total_runtime_matches_the_hand_summed_fixture() {
    assert_eq!(reacher().total_seconds, 23_550);
}

#[test]
fn measured_time_subtracts_only_what_was_measured() {
    let plan = reacher();
    assert_eq!(plan.measured_seconds(), 23_550 - 1_026);
}

#[test]
fn coverage_is_disclosed_not_extrapolated() {
    let note = reacher().coverage_note();

    assert!(note.contains("6 of 8"), "note must name the measured count: {note}");
    assert!(note.contains("75%"), "note must give the percentage: {note}");
}

#[test]
fn a_series_with_no_segment_data_says_so() {
    let plan = BingePlan {
        episodes: 10,
        total_seconds: 30_000,
        segments: SegmentStats { episodes: 10, measured: 0, skippable_seconds: 0 },
    };

    assert_eq!(plan.measured_seconds(), 30_000);
    assert!(plan.coverage_note().contains("No intro/outro data"));
}

#[test]
fn subtraction_can_never_go_negative() {
    let plan = BingePlan {
        episodes: 1,
        total_seconds: 100,
        segments: SegmentStats { episodes: 1, measured: 1, skippable_seconds: 500 },
    };

    assert_eq!(plan.measured_seconds(), 0);
}

#[test]
fn pacing_rounds_up_so_a_partial_day_still_counts() {
    let plan = reacher();

    assert_eq!(plan.days_at(2), 4);
    assert_eq!(plan.days_at(3), 3);
    assert_eq!(plan.days_at(1), 8);
    assert_eq!(plan.days_at(0), 0);
}

#[test]
fn durations_read_as_english() {
    assert_eq!(format_duration(23_550), "6 hours, 32 minutes");
    assert_eq!(format_duration(3_600), "1 hours");
    assert_eq!(format_duration(90), "1 minutes");
}
