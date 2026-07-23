//! Regression tests for family DS-1 — block enforcement.
//!
//! DS-1 was that `/block` was never *read* by `adopt`/`marry` (enforcement
//! absent) and `/unblock` never deleted the row. The end-to-end command paths
//! (`Marry::run` / `Adopt::run` and the accept handlers) and the `remove_block`
//! DELETE require a live `PgPool` + Discord interaction, for which this crate has
//! no test harness yet (see audit CC-6). These tests pin the pure guard the
//! enforcement now depends on — `FamilyRow::is_blocked` — which did not exist
//! before the fix, so a stored block was structurally unreadable.

use family::FamilyRow;
use serenity::all::UserId;

#[test]
fn blocked_target_reports_proposer_as_blocked() {
    // Models the DS-1 scenario at the predicate level: A blocks B, then B
    // proposes to A. Enforcement reads A's row and must see B as blocked.
    let mut blocker = FamilyRow::new(1.into(), 1.into(), "Alice".to_string());
    blocker.add_blocked(UserId::new(2));

    assert!(blocker.is_blocked(UserId::new(2)));
}

#[test]
fn unblocked_user_is_not_reported_blocked() {
    let blocker = FamilyRow::new(1.into(), 1.into(), "Alice".to_string());

    assert!(!blocker.is_blocked(UserId::new(2)));
}

#[test]
fn block_targets_only_the_named_user() {
    let mut blocker = FamilyRow::new(1.into(), 1.into(), "Alice".to_string());
    blocker.add_blocked(UserId::new(2));

    assert!(blocker.is_blocked(UserId::new(2)));
    assert!(!blocker.is_blocked(UserId::new(3)));
}
