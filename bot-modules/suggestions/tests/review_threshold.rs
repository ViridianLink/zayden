//! Regression coverage for the suggestions promote/demote threshold (DS-1).
//!
//! The demote branch once used `neg_count - pos_count <= 15`, inverting the sign
//! of the delta so a heavily downvoted suggestion was never removed from the
//! review channel, while suggestions in the hysteresis gap were spuriously
//! demoted. `review_action` now measures both thresholds against the same signed
//! `pos_count - neg_count` delta.

use suggestions::{ReviewAction, review_action};

#[test]
fn heavily_downvoted_post_is_demoted() {
    // delta = 5 - 25 = -20. Intended: delete the review post.
    // Pre-fix (`neg - pos = 20 <= 15` → false) this landed in the no-op gap and
    // the stale post lingered forever — the exact DS-1 failure scenario.
    assert_eq!(review_action(5, 25), ReviewAction::Demote);
}

#[test]
fn promotion_threshold_is_inclusive_at_20() {
    assert_eq!(review_action(25, 0), ReviewAction::Promote);
    assert_eq!(review_action(20, 0), ReviewAction::Promote); // boundary
}

#[test]
fn hysteresis_gap_leaves_post_unchanged() {
    // delta in [16, 19] is neither promote nor demote.
    // Pre-fix (`neg - pos <= 15` → e.g. -19 <= 15 → true) this spuriously demoted
    // a post that should persist.
    assert_eq!(review_action(19, 0), ReviewAction::Unchanged);
    assert_eq!(review_action(16, 0), ReviewAction::Unchanged);
}

#[test]
fn demote_threshold_is_inclusive_at_15() {
    assert_eq!(review_action(15, 0), ReviewAction::Demote); // boundary
    assert_eq!(review_action(0, 0), ReviewAction::Demote);
    assert_eq!(review_action(3, 20), ReviewAction::Demote); // delta -17
}
