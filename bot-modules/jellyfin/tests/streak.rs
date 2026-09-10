//! Streak arithmetic. The window is deliberately hand-built so the expected
//! numbers are computed by a human, not by the code under test.

use jellyfin::stats::streak::compute;
use jiff::Span;
use jiff::civil::{Date, date};

fn day(offset: i64) -> Date {
    date(2026, 9, 10) - Span::new().days(offset)
}

#[test]
fn an_empty_history_has_no_streak() {
    let stats = compute(&[], 0, day(0));

    assert_eq!(stats.current, 0);
    assert_eq!(stats.longest, 0);
    assert_eq!(stats.total_days, 0);
}

#[test]
fn a_run_ending_today_is_the_current_streak() {
    let days = [day(2), day(1), day(0)];
    let stats = compute(&days, 3600, day(0));

    assert_eq!(stats.current, 3);
    assert_eq!(stats.longest, 3);
    assert_eq!(stats.total_days, 3);
}

#[test]
fn yesterday_still_counts_so_a_streak_does_not_break_at_midnight() {
    // Before anyone has watched anything today, a live streak must not read 0.
    let days = [day(2), day(1)];
    let stats = compute(&days, 0, day(0));

    assert_eq!(stats.current, 2);
}

#[test]
fn a_gap_ends_the_current_streak() {
    let days = [day(9), day(8), day(7)];
    let stats = compute(&days, 0, day(0));

    assert_eq!(stats.current, 0);
    assert_eq!(stats.longest, 3);
}

#[test]
fn the_longest_run_survives_later_gaps() {
    // A hand-counted 400-day window: a 5-day run, a gap, then a 2-day run
    // ending yesterday.
    let days =
        [day(300), day(299), day(298), day(297), day(296), day(100), day(2), day(1)];
    let stats = compute(&days, 7_200, day(0));

    assert_eq!(stats.longest, 5);
    assert_eq!(stats.current, 2);
    assert_eq!(stats.total_days, 8);
}

#[test]
fn watching_twice_in_a_day_counts_once() {
    let days = [day(1), day(1), day(0), day(0), day(0)];
    let stats = compute(&days, 0, day(0));

    assert_eq!(stats.total_days, 2);
    assert_eq!(stats.current, 2);
}

#[test]
fn total_seconds_is_passed_through_untouched() {
    let stats = compute(&[day(0)], 12_345, day(0));
    assert_eq!(stats.total_seconds, 12_345);
}
