use std::time::Duration;

use music::embeds::{
    format_duration,
    parse_timestamp,
    progress_bar,
    queue_page_count,
};

/// Index of the '🔘' fill marker in a rendered bar, or `None` if it is absent
/// (which happens once the marker index reaches the bar width).
fn marker_index(bar: &str) -> Option<usize> {
    bar.chars().filter(|c| *c == '🔘' || *c == '▬').position(|c| c == '🔘')
}

#[test]
fn format_duration_under_an_hour_omits_hours() {
    assert_eq!(format_duration(Duration::from_secs(83)), "1:23");
    assert_eq!(format_duration(Duration::from_secs(5)), "0:05");
}

#[test]
fn format_duration_includes_hours_when_present() {
    assert_eq!(format_duration(Duration::from_secs(3723)), "1:02:03");
}

#[test]
fn parse_timestamp_accepts_plain_seconds() {
    assert_eq!(parse_timestamp("83"), Some(Duration::from_secs(83)));
}

#[test]
fn parse_timestamp_accepts_mm_ss() {
    assert_eq!(parse_timestamp("1:23"), Some(Duration::from_secs(83)));
}

#[test]
fn parse_timestamp_accepts_hh_mm_ss() {
    assert_eq!(parse_timestamp("1:02:03"), Some(Duration::from_secs(3723)));
}

#[test]
fn parse_timestamp_rejects_garbage() {
    assert_eq!(parse_timestamp("not-a-timestamp"), None);
    assert_eq!(parse_timestamp(""), None);
}

#[test]
fn queue_page_count_has_a_floor_of_one() {
    assert_eq!(queue_page_count(0), 1);
    assert_eq!(queue_page_count(1), 1);
}

#[test]
fn queue_page_count_rounds_up() {
    assert_eq!(queue_page_count(10), 1);
    assert_eq!(queue_page_count(11), 2);
    assert_eq!(queue_page_count(20), 2);
    assert_eq!(queue_page_count(21), 3);
}

#[test]
fn progress_bar_without_a_known_total_reports_playing() {
    let bar = progress_bar(Duration::from_secs(30), None);
    assert!(bar.contains("🔴 PLAYING"), "unexpected bar: {bar}");
}

#[test]
fn progress_bar_zero_total_puts_the_marker_at_the_start() {
    let bar = progress_bar(Duration::from_secs(5), Some(Duration::ZERO));
    assert_eq!(marker_index(&bar), Some(0));
}

#[test]
fn progress_bar_clamps_elapsed_past_the_total() {
    // Marker index saturates at the bar width, which renders with no '🔘'.
    let bar = progress_bar(Duration::from_secs(999), Some(Duration::from_secs(100)));
    assert_eq!(marker_index(&bar), None);
}

/// CC-3 replaced `(ratio * WIDTH).round() as u32` with integer round-half-up so
/// the float→int cast (and its lint suppression) could go away. These are the
/// positions the float version produced — including both sides of the rounding
/// boundaries, where a truncating rewrite would have drifted by one cell.
#[test]
fn progress_bar_rounds_to_the_nearest_cell() {
    // (elapsed secs, expected marker index) over a 3:33 track. 20 == no marker,
    // i.e. the bar is full.
    const TOTAL_SECS: u64 = 213;
    const CASES: &[(u64, usize)] = &[
        (0, 0),
        (5, 0),  // 0.469 rounds down
        (6, 1),  // 0.563 rounds up
        (53, 5), // 4.977 rounds up
        (106, 10),
        (107, 10),
        (160, 15),
        (202, 19), // 18.967 rounds up
        (208, 20), // 19.531 rounds up to a full bar
        (213, 20),
    ];

    let total = Duration::from_secs(TOTAL_SECS);

    for &(secs, expected) in CASES {
        let bar = progress_bar(Duration::from_secs(secs), Some(total));
        let actual = marker_index(&bar).unwrap_or(20);

        assert_eq!(
            actual, expected,
            "wrong marker cell at {secs}s of {TOTAL_SECS}s"
        );
    }
}
