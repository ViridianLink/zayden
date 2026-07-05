use std::time::Duration;

use music::embeds::{format_duration, parse_timestamp, queue_page_count};

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
