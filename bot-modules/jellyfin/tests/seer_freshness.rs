//! Regression tests for the Jellyseerr id cache window.
//!
//! `resolve` used to compute the cutoff as
//! `Timestamp::now() - Span::new().days(7)`. A `jiff::Timestamp` is a zoneless
//! instant, so jiff rejects calendar units in its arithmetic and the `Sub` impl
//! panics rather than returning an error. Every `/jellyfin request` therefore
//! killed its worker thread before reaching Jellyseerr.

use jellyfin::identity::seer_user::is_fresh;
use jiff::{SignedDuration, Timestamp};

fn ago(duration: SignedDuration) -> (Option<Timestamp>, Timestamp) {
    let now = Timestamp::now();
    (Some(now - duration), now)
}

#[test]
fn never_checked_is_stale() {
    assert!(!is_fresh(None, Timestamp::now()));
}

#[test]
fn just_checked_is_fresh() {
    let (checked, now) = ago(SignedDuration::from_secs(0));
    assert!(is_fresh(checked, now));
}

#[test]
fn inside_the_window_is_fresh() {
    let (checked, now) =
        ago(SignedDuration::from_hours(24 * 7) - SignedDuration::from_secs(1));
    assert!(is_fresh(checked, now));
}

#[test]
fn outside_the_window_is_stale() {
    let (checked, now) = ago(SignedDuration::from_hours(24 * 7));
    assert!(!is_fresh(checked, now));
}

#[test]
fn a_week_old_check_does_not_panic() {
    let (checked, now) = ago(SignedDuration::from_hours(24 * 30));
    assert!(!is_fresh(checked, now));
}
