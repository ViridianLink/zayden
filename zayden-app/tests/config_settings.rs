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
use zayden_app::config::tables::{
    AiSettingsRow,
    FamilySettingsRow,
    MusicSettingsRow,
};

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

// `music_settings.auto_disconnect_secs` moved from a Discord `Integer` option
// (range-checked by Discord itself) to a dashboard free-text field, so the
// normalisation the bot used to get for free now has to be explicit. These pin
// it: the editor can no longer write a value the inactivity check can't use.

#[test]
fn auto_disconnect_falls_back_to_the_default_on_junk() {
    for junk in ["", "   ", "abc", "12.5", "9999999999999999999"] {
        assert_eq!(
            MusicSettingsRow::parse_auto_disconnect_secs(junk),
            MusicSettingsRow::DEFAULT_AUTO_DISCONNECT_SECS,
            "{junk:?} should fall back to the default",
        );
    }
}

#[test]
fn auto_disconnect_clamps_into_range() {
    // Negative would become a huge `u64` at the `as_u64` cast in the music
    // module's `session_request`, effectively disabling auto-disconnect.
    assert_eq!(MusicSettingsRow::parse_auto_disconnect_secs("-1"), 0);
    assert_eq!(
        MusicSettingsRow::parse_auto_disconnect_secs("999999999"),
        MusicSettingsRow::MAX_AUTO_DISCONNECT_SECS,
    );
    assert_eq!(
        MusicSettingsRow::parse_auto_disconnect_secs("86400"),
        MusicSettingsRow::MAX_AUTO_DISCONNECT_SECS,
    );
}

#[test]
fn auto_disconnect_accepts_in_range_values() {
    assert_eq!(MusicSettingsRow::parse_auto_disconnect_secs("0"), 0);
    assert_eq!(MusicSettingsRow::parse_auto_disconnect_secs(" 300 "), 300);

    let max = MusicSettingsRow::MAX_AUTO_DISCONNECT_SECS;
    assert_eq!(MusicSettingsRow::parse_auto_disconnect_secs(&max.to_string()), max);
    assert!(max >= MusicSettingsRow::DEFAULT_AUTO_DISCONNECT_SECS);
}

#[test]
fn music_settings_empty_default_matches_the_parse_fallback() {
    let row = MusicSettingsRow::empty(123);

    assert_eq!(row.guild_id, 123);
    assert_eq!(
        row.auto_disconnect_secs,
        MusicSettingsRow::DEFAULT_AUTO_DISCONNECT_SECS,
    );
    assert_eq!(MusicSettingsRow::TABLE, "music_settings");
}

// music DS-3: announcements are opt-out (the column defaults to TRUE) and have
// no dedicated channel until an admin picks one, in which case the bot falls
// back to the channel the play command was used in.
#[test]
fn music_settings_empty_announces_to_the_command_channel() {
    let row = MusicSettingsRow::empty(123);

    assert!(row.announce_now_playing);
    assert_eq!(row.announce_channel_id, None);
}

// `ai_settings` replaced a hardcoded channel constant in the AI binding, so
// `responds_in` is now the only thing standing between a mention and a paid
// model call. Every reply costs money, so the off states matter more than the
// on state.

#[test]
fn ai_settings_empty_is_off_everywhere() {
    let row = AiSettingsRow::empty(123);

    assert_eq!(row.guild_id, 123);
    // Must match the `ai_settings.enabled` column DEFAULT (FALSE): a guild that
    // has never touched the dashboard gets no AI replies and no model spend.
    assert!(!row.enabled);
    assert_eq!(row.channel_id, None);
    assert!(!row.responds_in(456));
    assert_eq!(AiSettingsRow::TABLE, "ai_settings");
}

#[test]
fn ai_disabled_ignores_its_configured_channel() {
    let row = AiSettingsRow { guild_id: 123, enabled: false, channel_id: Some(456) };

    assert!(!row.responds_in(456));
}

#[test]
fn ai_enabled_without_a_channel_answers_anywhere() {
    let row = AiSettingsRow { guild_id: 123, enabled: true, channel_id: None };

    assert!(row.responds_in(456));
    assert!(row.responds_in(789));
}

#[test]
fn ai_enabled_with_a_channel_answers_only_there() {
    let row = AiSettingsRow { guild_id: 123, enabled: true, channel_id: Some(456) };

    assert!(row.responds_in(456));
    assert!(!row.responds_in(789));
}
