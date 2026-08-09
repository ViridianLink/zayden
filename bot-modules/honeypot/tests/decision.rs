//! The honeypot's decision/effect split (finding #11).
//!
//! `message_create` is the one path in the workspace that bans a member with no
//! human in the loop, and serenity's `Http` is a concrete type with no seam to
//! inject — so the ban/unban sequence cannot be driven end-to-end from a test.
//! Instead every branch of that sequence is extracted into a pure function and
//! the executor is reduced to a thin, straight-line caller. These pin what a
//! regression in that executor could get wrong:
//!
//! - `is_decoy_hit` — arming decides which channel (or none) is a trap.
//! - `decide` — an exempt member must be `Spare`d, never `Ban`ned.
//! - `outcome_of` — the honeypot #1 hazard: recording `SoftBanned` when the unban
//!   failed and the ban is in fact still standing.
//!
//! The exemption *matrix* itself is `tests/policy.rs`; here we only pin that
//! `decide` routes on it and threads the purge window through.

use std::collections::HashMap;

use honeypot::HoneypotOutcome;
use honeypot::message_create::{Action, decide, is_decoy_hit, outcome_of};
use honeypot::policy::{ExemptionPolicy, GuildFacts};
use serenity::all::{ChannelId, Permissions, RoleId, UserId};
use zayden_core::as_i64;

const GUILD: u64 = 100;
const OWNER: u64 = 1;
const STRANGER: u64 = 2;
const DECOY_CHANNEL: u64 = 555;
const OTHER_CHANNEL: u64 = 556;
const TRUSTED_ROLE: u64 = 12;
const PURGE: u32 = 3600;

fn facts() -> GuildFacts {
    let everyone_role = RoleId::new(GUILD);

    GuildFacts {
        owner_id: UserId::new(OWNER),
        role_perms: HashMap::from([(everyone_role, Permissions::empty())]),
        everyone_role,
    }
}

/// Only the owner is exempt — the policy a fresh guild gets.
const fn default_policy() -> ExemptionPolicy {
    ExemptionPolicy { exempt_admins: false, exempt_role_id: None }
}

#[test]
fn a_disarmed_guild_registers_no_hit() {
    // `channel_id == None` means the trap is off; no channel can match it.
    assert!(!is_decoy_hit(ChannelId::new(DECOY_CHANNEL), None));
    assert!(!is_decoy_hit(ChannelId::new(OTHER_CHANNEL), None));
}

#[test]
fn only_the_armed_channel_registers_a_hit() {
    let armed = Some(as_i64(DECOY_CHANNEL));

    assert!(is_decoy_hit(ChannelId::new(DECOY_CHANNEL), armed));
    assert!(
        !is_decoy_hit(ChannelId::new(OTHER_CHANNEL), armed),
        "a message outside the decoy channel must not trip the trap"
    );
}

#[test]
fn an_exempt_member_is_spared_not_banned() {
    // The owner is exempt under every policy (policy.rs pins the full matrix);
    // this pins that `decide` turns an exemption into `Spare`, never `Ban`.
    let action = decide(UserId::new(OWNER), &[], &facts(), &default_policy(), PURGE);

    assert_eq!(action, Action::Spare);
}

#[test]
fn the_exempt_role_routes_to_spare() {
    let policy = ExemptionPolicy {
        exempt_admins: false,
        exempt_role_id: Some(RoleId::new(TRUSTED_ROLE)),
    };

    let action = decide(
        UserId::new(STRANGER),
        &[RoleId::new(TRUSTED_ROLE)],
        &facts(),
        &policy,
        PURGE,
    );

    assert_eq!(action, Action::Spare);
}

#[test]
fn a_stranger_is_banned_with_the_configured_purge_window() {
    let action =
        decide(UserId::new(STRANGER), &[], &facts(), &default_policy(), PURGE);

    // The purge window the caller passed must reach the ban unchanged — a
    // dropped or defaulted window would purge the wrong amount of history.
    assert_eq!(action, Action::Ban { purge_seconds: PURGE });
}

// honeypot #1: the recorded outcome must match reality. A successful unban is a
// soft-ban; a failed unban leaves the ban standing, and recording *that* as
// `SoftBanned` would tell a moderator the member is back when they are not.
#[test]
fn a_successful_unban_is_recorded_as_a_soft_ban() {
    let unban: Result<(), ()> = Ok(());

    assert_eq!(outcome_of(&unban), HoneypotOutcome::SoftBanned);
}

#[test]
fn a_failed_unban_is_recorded_as_a_standing_ban() {
    let unban: Result<(), ()> = Err(());

    assert_eq!(outcome_of(&unban), HoneypotOutcome::BanStanding);
}
