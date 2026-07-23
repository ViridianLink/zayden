//! Pure-logic coverage for `FamilyRow` / `FamilySettings`, the row types moved
//! into the crate as concrete `PgPool` associated functions during the CC-1
//! migration (the generic `FamilyManager<Db>` trait was removed). These pin the
//! in-memory relationship/limit predicates the command layer relies on, so the
//! refactor is proven behaviour-preserving without a live DB.
//!
//! DB-touching paths (`FamilyRow::get`/`save`/`tree`, `FamilySettings::get`)
//! need a test-pool harness and are deferred to CC-6.

use family::{FamilyRow, FamilySettings, Relationships};
use serenity::all::UserId;

fn row_with(
    partner_ids: Vec<i64>,
    parent_ids: Vec<i64>,
    children_ids: Vec<i64>,
) -> FamilyRow {
    FamilyRow { partner_ids, parent_ids, children_ids, ..Default::default() }
}

#[test]
fn relationship_classifies_each_edge() {
    let row = row_with(vec![10], vec![20], vec![30]);

    assert_eq!(row.relationship(UserId::new(10)), Relationships::Partner);
    assert_eq!(row.relationship(UserId::new(20)), Relationships::Parent);
    assert_eq!(row.relationship(UserId::new(30)), Relationships::Child);
    assert_eq!(row.relationship(UserId::new(99)), Relationships::None);
}

#[test]
fn partner_takes_precedence_over_other_edges() {
    // A user appearing in multiple lists resolves as Partner first (matches the
    // if/else-if ordering the command checks depend on).
    let row = row_with(vec![10], vec![10], vec![10]);
    assert_eq!(row.relationship(UserId::new(10)), Relationships::Partner);
}

#[test]
fn block_helpers_round_trip() {
    let mut row = FamilyRow::default();
    assert!(!row.is_blocked(UserId::new(7)));

    row.add_blocked(UserId::new(7));
    assert!(row.is_blocked(UserId::new(7)));
    assert!(!row.is_blocked(UserId::new(8)));
}

#[test]
fn at_partner_limit_is_inclusive() {
    let row = row_with(vec![1], Vec::new(), Vec::new());
    assert!(row.at_partner_limit(1)); // one partner, limit one → at limit
    assert!(!row.at_partner_limit(2));

    let empty = FamilyRow::default();
    assert!(!empty.at_partner_limit(1));
}

#[test]
fn is_adopted_tracks_parents() {
    assert!(!FamilyRow::default().is_adopted());
    assert!(row_with(Vec::new(), vec![20], Vec::new()).is_adopted());
}

#[test]
fn settings_default_and_clamp() {
    assert_eq!(FamilySettings::default().max_partners(), 1);
    assert_eq!(FamilySettings::new(3).max_partners(), 3);
    // A negative configured value clamps to zero rather than panicking.
    assert_eq!(FamilySettings::new(-5).max_partners(), 0);
}
