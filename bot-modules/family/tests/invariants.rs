//! Regression tests for family DS-2 — accept-time invariant re-checks — under
//! the per-guild design change.
//!
//! DS-2 was that the `marry`/`adopt` **accept** handlers ran no invariant
//! checks: the partner-cap / "already adopted" / "already related" guards lived
//! only in the *command* (proposal time), so two pending proposals for the same
//! person, both accepted, bypassed them (a partner past the cap, or a child with
//! two parents).
//!
//! With the guild-scope change the cap is now the guild's configured
//! `family_settings.max_partners` (no longer a const), so the accept handler
//! re-checks against that value. The end-to-end accept paths
//! (`components::{marry,adopt}::accept`) require a live `PgPool` + Discord
//! interaction, for which this crate has no test harness (see audit CC-6). These
//! tests pin the pure guards the accept re-check depends on —
//! `FamilyRow::at_partner_limit` (now taking the configured max) and
//! `is_adopted` — plus `FamilySettings::max_partners` clamping. The residual
//! *same-tick concurrent* double-accept remains the CC-9 atomic-write class,
//! folded into the CC-1 concrete-`PgPool` migration.

use family::{FamilyRow, FamilySettings};

#[test]
fn user_with_a_partner_is_at_the_default_limit() {
    // Models the DS-2 marry scenario: Z accepted X's proposal (one partner).
    // When Z accepts Y's stale proposal, the accept handler re-reads Z's row and
    // must see the guild's cap (default 1) already reached.
    let mut z = FamilyRow::new(1.into(), 1.into(), "Z".to_string());
    let x = FamilyRow::new(1.into(), 2.into(), "X".to_string());
    z.add_partner(&x);

    assert!(z.at_partner_limit(FamilySettings::default().max_partners()));
}

#[test]
fn user_with_no_partner_is_not_at_the_limit() {
    let z = FamilyRow::new(1.into(), 1.into(), "Z".to_string());

    assert!(!z.at_partner_limit(FamilySettings::default().max_partners()));
}

#[test]
fn a_higher_configured_cap_permits_another_partner() {
    // A guild that raised max_partners to 2 must NOT report a one-partner user as
    // at the limit — the accept re-check honours the per-guild setting.
    let mut z = FamilyRow::new(1.into(), 1.into(), "Z".to_string());
    let x = FamilyRow::new(1.into(), 2.into(), "X".to_string());
    z.add_partner(&x);

    let settings = FamilySettings::new(2);
    assert!(!z.at_partner_limit(settings.max_partners()));
}

#[test]
fn negative_configured_cap_clamps_to_zero() {
    // A nonsensical negative configuration must not wrap to a huge usize; it
    // clamps to 0 so anyone is "at the limit".
    let settings = FamilySettings::new(-5);

    assert_eq!(settings.max_partners(), 0);
    assert!(FamilyRow::new(1.into(), 1.into(), "Z".to_string()).at_partner_limit(0));
}

#[test]
fn user_with_a_parent_is_already_adopted() {
    // Models the DS-2 adopt scenario: a child who accepted one adoption now has
    // a parent, so accepting a second pending adoption must be rejected.
    let mut child = FamilyRow::new(1.into(), 1.into(), "Kid".to_string());
    let parent = FamilyRow::new(1.into(), 2.into(), "Parent".to_string());
    child.add_parent(&parent);

    assert!(child.is_adopted());
}

#[test]
fn user_with_no_parent_is_not_adopted() {
    let child = FamilyRow::new(1.into(), 1.into(), "Kid".to_string());

    assert!(!child.is_adopted());
}
