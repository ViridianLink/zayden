//! Coverage for the guild-settings config rows surfaced by the dashboard.
//!
//! `family_settings` (migration `0015_family_guild_scope`) shipped bot-side only;
//! the dashboard editor (`save_family_settings`) is built on the `FamilySettingsRow`
//! `SettingsRow`. The one invariant a pure test can pin without a DB is that the
//! editor's *unsaved* default matches what the bot enforces for a guild with no
//! row — otherwise the dashboard would advertise a partner limit the bot ignores.
//! The `select`/`upsert` round-trip is DB-bound and covered like the other stores
//! by manual checks against a live Postgres (see `tests/entitlement.rs`).

use zayden_app::config::SettingsRow;
use zayden_app::config::tables::FamilySettingsRow;

#[test]
fn family_settings_empty_default_matches_enforcement() {
    let row = FamilySettingsRow::empty(123);

    assert_eq!(row.guild_id, 123);
    // Must equal the `family_settings.max_partners` column DEFAULT and the family
    // module's `FamilySettings::default().max_partners`, both `1`.
    assert_eq!(row.max_partners, 1);
}

#[test]
fn family_settings_targets_the_correct_table() {
    assert_eq!(FamilySettingsRow::TABLE, "family_settings");
}
