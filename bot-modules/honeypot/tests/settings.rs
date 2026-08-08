//! The single normalisation contract for the `honeypot_settings` write.
//!
//! Two editors reach this table — `/honeypot set|disable` in Discord and the
//! dashboard's honeypot form. The owner's ruling on
//! [honeypot #5](../../../design-docs/audits/honeypot.md) was to keep both
//! surfaces and converge them on one owner, so what these tests pin is the
//! *contract*, not either caller: the form-field parsing rule, and the
//! invariant that arming from Discord never disturbs exemptions the dashboard
//! set.
//!
//! Both are offline — `HoneypotConfig` is a pure value type, so none of this
//! needs a `DATABASE_URL`.

use honeypot::HoneypotError;
use honeypot::settings::HoneypotConfig;
use serenity::all::{ChannelId, RoleId};
use zayden_app::config::{HoneypotSettingsRow, SettingsRow};
use zayden_core::as_i64;

const GUILD: u64 = 100;
const CHANNEL: u64 = 555_000_000_000_000_001;
const ROLE: u64 = 777_000_000_000_000_002;

const fn armed_row() -> HoneypotSettingsRow {
    HoneypotSettingsRow {
        guild_id: as_i64(GUILD),
        channel_id: Some(as_i64(CHANNEL)),
        exempt_admins: true,
        exempt_role_id: Some(as_i64(ROLE)),
        purge_seconds: HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
    }
}

/// The purge field as the form submits it when the admin has not touched it.
/// Empty parses to the default, so the snowflake tests below stay about
/// snowflakes.
const UNSET_PURGE: &str = "";

// --- the form-field parsing rule ------------------------------------------
//
// This is the rule the two writers disagreed on. The dashboard used to parse
// its form fields with `s.trim().parse().ok()`, which maps *both* "the admin
// cleared this field" and "this field is garbage" to `None`. For a trap that
// auto-bans, those must not be the same outcome: a garbled channel id silently
// disarmed the honeypot and reported success.

#[test]
fn a_blank_field_clears_the_setting() {
    let config = HoneypotConfig::from_form("", false, "  ", UNSET_PURGE).unwrap();

    assert_eq!(config.channel_id, None);
    assert_eq!(config.exempt_role_id, None);
    assert!(!config.is_armed());
}

#[test]
fn a_populated_field_parses_to_a_snowflake() {
    let config = HoneypotConfig::from_form(
        &CHANNEL.to_string(),
        true,
        &ROLE.to_string(),
        UNSET_PURGE,
    )
    .unwrap();

    assert_eq!(config.channel_id, Some(ChannelId::new(CHANNEL)));
    assert_eq!(config.exempt_role_id, Some(RoleId::new(ROLE)));
    assert!(config.exempt_admins);
    assert!(config.is_armed());
}

// The regression. Before the shared owner these two assertions were the
// difference between "trap disarmed, success reported" and an error.
#[test]
fn a_garbled_channel_is_an_error_not_a_silent_disarm() {
    let err = HoneypotConfig::from_form("not-a-snowflake", false, "", UNSET_PURGE)
        .expect_err("a malformed channel id must not parse");

    assert!(
        matches!(err, HoneypotError::InvalidSnowflake { field, .. } if field == "channel"),
        "expected InvalidSnowflake for the channel field, got {err:?}",
    );
}

#[test]
fn a_garbled_exempt_role_is_an_error_not_a_silent_clear() {
    let err =
        HoneypotConfig::from_form(&CHANNEL.to_string(), false, "12x34", UNSET_PURGE)
            .expect_err("a malformed role id must not parse");

    assert!(
        matches!(err, HoneypotError::InvalidSnowflake { field, .. } if field == "exempt role"),
        "expected InvalidSnowflake for the exempt role field, got {err:?}",
    );
}

// A negative number parses as `i64` but is not a snowflake. The old `.ok()`
// path would have accepted it through `parse::<i64>()` and written it to the
// column; parsing as `u64` is what rejects it.
#[test]
fn a_negative_id_is_rejected() {
    let err = HoneypotConfig::from_form("-1", false, "", UNSET_PURGE)
        .expect_err("a negative channel id must not parse");

    assert!(matches!(err, HoneypotError::InvalidSnowflake { .. }), "got {err:?}");
}

#[test]
fn an_invalid_field_is_reported_with_its_value() {
    let err = HoneypotConfig::from_form("oops", false, "", UNSET_PURGE).unwrap_err();

    // The admin needs to see what was rejected, and the message is user-facing.
    assert!(err.to_string().contains("oops"), "got {err}");
}

// --- the column mapping ----------------------------------------------------

#[test]
fn a_row_round_trips_through_the_config() {
    let config = HoneypotConfig::from(&armed_row());

    assert_eq!(config.channel_id, Some(ChannelId::new(CHANNEL)));
    assert_eq!(config.exempt_role_id, Some(RoleId::new(ROLE)));
    assert!(config.exempt_admins);

    let mut row = HoneypotSettingsRow::empty(as_i64(GUILD));
    config.apply(&mut row);

    assert_eq!(row.channel_id, Some(as_i64(CHANNEL)));
    assert_eq!(row.exempt_role_id, Some(as_i64(ROLE)));
    assert!(row.exempt_admins);
    assert_eq!(row.purge_seconds, HoneypotSettingsRow::DEFAULT_PURGE_SECONDS);
}

#[test]
fn an_empty_row_is_a_disarmed_config() {
    let config = HoneypotConfig::from(&HoneypotSettingsRow::empty(as_i64(GUILD)));

    assert_eq!(config, HoneypotConfig::default());
    assert!(!config.is_armed());
}

// --- the invariant that lets both editors coexist --------------------------
//
// `/honeypot set` is a channel-only edit. If it ever wrote a whole config it
// would clobber exemptions the dashboard owns — which is the divergence
// finding #5 is about. These pin that `arm`/`disarm` are field-scoped.

#[test]
fn arming_preserves_the_exemption_policy() {
    let mut row = armed_row();
    row.channel_id = None;

    let new_channel = ChannelId::new(999_000_000_000_000_003);
    HoneypotConfig::arm_row(&mut row, new_channel);

    assert_eq!(row.channel_id, Some(as_i64(new_channel.get())));
    assert!(row.exempt_admins, "arming must not clear exempt_admins");
    assert_eq!(
        row.exempt_role_id,
        Some(as_i64(ROLE)),
        "arming must not clear the exempt role",
    );
}

#[test]
fn disarming_preserves_the_exemption_policy_so_re_arming_restores_it() {
    let mut row = armed_row();

    HoneypotConfig::disarm_row(&mut row);

    assert_eq!(row.channel_id, None);
    assert!(row.exempt_admins, "disarming must not clear exempt_admins");
    assert_eq!(
        row.exempt_role_id,
        Some(as_i64(ROLE)),
        "disarming must not clear the exempt role",
    );
}

// --- the purge window (finding #9) -----------------------------------------
//
// The window used to be a hardcoded 24 h const. It is the trap's most
// destructive parameter — it deletes the offender's messages *server-wide*, not
// just in the decoy channel — so the thing worth pinning is not that it is
// configurable but that it can never leave Discord's accepted range, whatever
// an admin types into a free-text form field.

#[test]
fn an_untouched_purge_field_keeps_the_previous_hardcoded_window() {
    // The migration defaults the column to this, so arming a honeypot must
    // behave exactly as it did before the column existed.
    assert_eq!(HoneypotSettingsRow::DEFAULT_PURGE_SECONDS, 24 * 60 * 60);
    assert_eq!(
        HoneypotSettingsRow::parse_purge_seconds(UNSET_PURGE),
        HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
    );
    assert_eq!(
        HoneypotSettingsRow::empty(as_i64(GUILD)).purge_seconds,
        HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
    );
}

#[test]
fn a_purge_window_is_parsed_and_surrounding_space_ignored() {
    assert_eq!(HoneypotSettingsRow::parse_purge_seconds("3600"), 3600);
    assert_eq!(HoneypotSettingsRow::parse_purge_seconds("  3600  "), 3600);
}

// Zero is a real choice ("ban but keep their history"), not a missing value —
// it must survive rather than fall back to the default.
#[test]
fn zero_is_a_kept_value_not_a_missing_one() {
    assert_eq!(HoneypotSettingsRow::parse_purge_seconds("0"), 0);
}

// Discord rejects `delete_message_seconds` above 7 days, so an over-large entry
// has to be clamped before it reaches the API rather than failing the ban — the
// ban is the part that matters, the window is the preference.
#[test]
fn an_over_large_window_is_clamped_to_discords_ceiling() {
    assert_eq!(HoneypotSettingsRow::MAX_PURGE_SECONDS, 7 * 24 * 60 * 60);

    assert_eq!(
        HoneypotSettingsRow::parse_purge_seconds("999999999"),
        HoneypotSettingsRow::MAX_PURGE_SECONDS,
    );
}

// A negative window would become a huge `u32` if it were cast rather than
// clamped, asking Discord to purge ~136 years of history.
#[test]
fn a_negative_window_cannot_become_a_huge_unsigned_one() {
    assert_eq!(HoneypotSettingsRow::parse_purge_seconds("-1"), 0);

    let row = HoneypotSettingsRow {
        purge_seconds: -1,
        ..HoneypotSettingsRow::empty(as_i64(GUILD))
    };
    assert_eq!(row.purge_seconds_u32(), 0);
}

// The row-level accessor re-clamps rather than trusting the column, because the
// `CHECK` constraint only landed with `0024_honeypot_purge` — a row written
// before it must still be safe to hand to `ban`.
#[test]
fn the_accessor_re_clamps_a_row_that_predates_the_check_constraint() {
    let row = HoneypotSettingsRow {
        purge_seconds: i32::MAX,
        ..HoneypotSettingsRow::empty(as_i64(GUILD))
    };

    assert_eq!(
        row.purge_seconds_u32(),
        u32::try_from(HoneypotSettingsRow::MAX_PURGE_SECONDS).unwrap(),
    );
}

#[test]
fn garbage_falls_back_to_the_default_rather_than_disabling_the_purge() {
    // Contrast the snowflake fields above, where garbage is an error: an
    // unparseable *window* must not fail the save, because the safe reading is
    // "the admin did not mean to change this", not "purge nothing".
    assert_eq!(
        HoneypotSettingsRow::parse_purge_seconds("oops"),
        HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
    );
}

#[test]
fn the_purge_window_round_trips_through_the_form_and_the_row() {
    let config =
        HoneypotConfig::from_form(&CHANNEL.to_string(), false, "", "3600").unwrap();

    assert_eq!(config.purge_seconds, 3600);
    assert_eq!(config.purge_seconds_u32(), 3600);

    let mut row = HoneypotSettingsRow::empty(as_i64(GUILD));
    config.apply(&mut row);

    assert_eq!(row.purge_seconds, 3600);
    assert_eq!(HoneypotConfig::from(&row).purge_seconds, 3600);
}

// `HoneypotConfig::default()` is hand-written rather than derived precisely so
// this holds — a derived `Default` would give `0`, silently meaning "purge
// nothing" for any caller that starts from a default config.
#[test]
fn a_default_config_purges_the_default_window_not_nothing() {
    assert_eq!(
        HoneypotConfig::default().purge_seconds,
        HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
    );
}
