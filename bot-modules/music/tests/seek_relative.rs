//! Signed seek parsing, which replaced the `forward` and `rewind` subcommands.
//!
//! Those two were deleted to free slots under Discord's 25-subcommand cap, so
//! `+30` / `-30` on `/music seek` is now the *only* way to seek relatively. A
//! regression here silently removes functionality rather than failing loudly.

use std::time::Duration;

use music::{SeekTarget, parse_seek, parse_timestamp};

const fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn bare_values_are_absolute() {
    assert_eq!(parse_seek("83"), Some(SeekTarget::Absolute(secs(83))));
    assert_eq!(parse_seek("1:23"), Some(SeekTarget::Absolute(secs(83))));
    assert_eq!(parse_seek("1:00:00"), Some(SeekTarget::Absolute(secs(3_600))));
    assert_eq!(parse_seek("0"), Some(SeekTarget::Absolute(Duration::ZERO)));
}

#[test]
fn a_leading_plus_seeks_forward() {
    assert_eq!(parse_seek("+30"), Some(SeekTarget::Forward(secs(30))));
    assert_eq!(parse_seek("+1:30"), Some(SeekTarget::Forward(secs(90))));
}

#[test]
fn a_leading_minus_seeks_backward() {
    assert_eq!(parse_seek("-30"), Some(SeekTarget::Backward(secs(30))));
    assert_eq!(parse_seek("-1:30"), Some(SeekTarget::Backward(secs(90))));
}

#[test]
fn surrounding_whitespace_is_tolerated() {
    assert_eq!(parse_seek("  +30 "), Some(SeekTarget::Forward(secs(30))));
    assert_eq!(parse_seek(" 1:23"), Some(SeekTarget::Absolute(secs(83))));
}

#[test]
fn malformed_input_is_rejected() {
    for input in ["", "+", "-", "abc", "1:", ":30", "1:2:3:4:x", "+ 30", "--30"] {
        assert_eq!(parse_seek(input), None, "`{input}` should not parse");
    }
}

#[test]
fn rewinding_past_the_start_saturates_at_zero() {
    // `seek::run` applies the offset with `saturating_sub`; underflow here would
    // panic in debug and wrap to a ~584-year seek in release.
    let elapsed = secs(10);
    let Some(SeekTarget::Backward(offset)) = parse_seek("-30") else {
        panic!("expected a backward seek");
    };

    assert_eq!(elapsed.saturating_sub(offset), Duration::ZERO);
}

#[test]
fn absolute_parsing_is_unchanged_by_the_signed_wrapper() {
    // `parse_timestamp` is still used directly elsewhere; the sign handling must
    // live entirely in `parse_seek`.
    assert_eq!(parse_timestamp("1:23"), Some(secs(83)));
    assert_eq!(parse_timestamp("+30"), None);
    assert_eq!(parse_timestamp("-30"), None);
}
