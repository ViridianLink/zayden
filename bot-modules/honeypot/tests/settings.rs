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
    }
}

// --- the form-field parsing rule ------------------------------------------
//
// This is the rule the two writers disagreed on. The dashboard used to parse
// its form fields with `s.trim().parse().ok()`, which maps *both* "the admin
// cleared this field" and "this field is garbage" to `None`. For a trap that
// auto-bans, those must not be the same outcome: a garbled channel id silently
// disarmed the honeypot and reported success.

#[test]
fn a_blank_field_clears_the_setting() {
    let config = HoneypotConfig::from_form("", false, "  ").unwrap();

    assert_eq!(config.channel_id, None);
    assert_eq!(config.exempt_role_id, None);
    assert!(!config.is_armed());
}

#[test]
fn a_populated_field_parses_to_a_snowflake() {
    let config =
        HoneypotConfig::from_form(&CHANNEL.to_string(), true, &ROLE.to_string())
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
    let err = HoneypotConfig::from_form("not-a-snowflake", false, "")
        .expect_err("a malformed channel id must not parse");

    assert!(
        matches!(err, HoneypotError::InvalidSnowflake { field, .. } if field == "channel"),
        "expected InvalidSnowflake for the channel field, got {err:?}",
    );
}

#[test]
fn a_garbled_exempt_role_is_an_error_not_a_silent_clear() {
    let err = HoneypotConfig::from_form(&CHANNEL.to_string(), false, "12x34")
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
    let err = HoneypotConfig::from_form("-1", false, "")
        .expect_err("a negative channel id must not parse");

    assert!(matches!(err, HoneypotError::InvalidSnowflake { .. }), "got {err:?}");
}

#[test]
fn an_invalid_field_is_reported_with_its_value() {
    let err = HoneypotConfig::from_form("oops", false, "").unwrap_err();

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
