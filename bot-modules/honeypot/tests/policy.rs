//! Exemption matrix for the honeypot trap.
//!
//! The trap's whole job is to ban strangers on their first message, so the
//! exemption check is the only thing standing between a mis-set flag and a
//! self-inflicted staff ban. These pin the decision the user signed off on:
//! the guild owner is always safe, and **nothing else is** unless a guild
//! explicitly opts in.

use std::collections::HashMap;

use honeypot::policy::{ExemptionPolicy, GuildFacts, guild_permissions, is_exempt};
use serenity::all::{Permissions, RoleId, UserId};
use zayden_app::config::{HoneypotSettingsRow, SettingsRow};
use zayden_core::as_i64;

const GUILD: u64 = 100;
const OWNER: u64 = 1;
const MEMBER: u64 = 2;

const ADMIN_ROLE: u64 = 10;
const MOD_ROLE: u64 = 11;
const TRUSTED_ROLE: u64 = 12;
const PLAIN_ROLE: u64 = 13;

fn facts(everyone: Permissions) -> GuildFacts {
    let everyone_role = RoleId::new(GUILD);

    let role_perms = HashMap::from([
        (everyone_role, everyone),
        (RoleId::new(ADMIN_ROLE), Permissions::ADMINISTRATOR),
        (RoleId::new(MOD_ROLE), Permissions::MANAGE_GUILD),
        (RoleId::new(TRUSTED_ROLE), Permissions::empty()),
        (RoleId::new(PLAIN_ROLE), Permissions::SEND_MESSAGES),
    ]);

    GuildFacts { owner_id: UserId::new(OWNER), role_perms, everyone_role }
}

/// The policy a guild gets when it has never touched the dashboard.
fn default_policy() -> ExemptionPolicy {
    ExemptionPolicy::from(&HoneypotSettingsRow::empty(as_i64(GUILD)))
}

fn exempt(user: u64, roles: &[u64], policy: &ExemptionPolicy) -> bool {
    exempt_with(user, roles, Permissions::empty(), policy)
}

fn exempt_with(
    user: u64,
    roles: &[u64],
    everyone: Permissions,
    policy: &ExemptionPolicy,
) -> bool {
    let roles: Vec<RoleId> = roles.iter().copied().map(RoleId::new).collect();
    is_exempt(UserId::new(user), &roles, &facts(everyone), policy)
}

#[test]
fn default_policy_exempts_only_the_owner() {
    let policy = default_policy();

    assert_eq!(policy, ExemptionPolicy {
        exempt_admins: false,
        exempt_role_id: None
    });
    assert!(exempt(OWNER, &[], &policy));
}

#[test]
fn a_plain_member_is_never_exempt() {
    assert!(!exempt(MEMBER, &[], &default_policy()));
    assert!(!exempt(MEMBER, &[PLAIN_ROLE], &default_policy()));
}

// The user chose "guild owner only by default", so an admin walking into the
// trap on a fresh install is *supposed* to be caught. If this flips, every
// guild silently loses the trap against a compromised staff account.
#[test]
fn admins_are_not_exempt_by_default() {
    assert!(!exempt(MEMBER, &[ADMIN_ROLE], &default_policy()));
    assert!(!exempt(MEMBER, &[MOD_ROLE], &default_policy()));
}

#[test]
fn admins_are_exempt_once_opted_in() {
    let policy = ExemptionPolicy { exempt_admins: true, exempt_role_id: None };

    assert!(exempt(MEMBER, &[ADMIN_ROLE], &policy));
    // Manage Server is treated the same as Administrator, matching the
    // permission the dashboard itself gates guild administration on.
    assert!(exempt(MEMBER, &[MOD_ROLE], &policy));
    assert!(!exempt(MEMBER, &[PLAIN_ROLE], &policy));
}

// `@everyone` is a role like any other and its id is the guild id, not a role in
// the member's own list — miss it and a guild that grants Administrator server
// -wide would still have its staff soft-banned.
#[test]
fn everyone_granted_permissions_count() {
    let policy = ExemptionPolicy { exempt_admins: true, exempt_role_id: None };

    assert!(exempt_with(MEMBER, &[], Permissions::ADMINISTRATOR, &policy));
    assert!(exempt_with(MEMBER, &[], Permissions::MANAGE_GUILD, &policy));
    assert!(!exempt_with(MEMBER, &[], Permissions::SEND_MESSAGES, &policy));
}

#[test]
fn the_exempt_role_is_honoured() {
    let policy = ExemptionPolicy {
        exempt_admins: false,
        exempt_role_id: Some(RoleId::new(TRUSTED_ROLE)),
    };

    // The role carries no permissions at all — it exempts purely by identity.
    assert!(exempt(MEMBER, &[TRUSTED_ROLE], &policy));
    assert!(!exempt(MEMBER, &[PLAIN_ROLE], &policy));
    assert!(!exempt(MEMBER, &[], &policy));
}

#[test]
fn the_owner_is_exempt_regardless_of_policy() {
    for policy in [
        default_policy(),
        ExemptionPolicy { exempt_admins: true, exempt_role_id: None },
        ExemptionPolicy {
            exempt_admins: false,
            exempt_role_id: Some(RoleId::new(TRUSTED_ROLE)),
        },
    ] {
        assert!(exempt(OWNER, &[], &policy));
    }
}

#[test]
fn permissions_accumulate_across_roles() {
    let facts = facts(Permissions::SEND_MESSAGES);
    let roles = [RoleId::new(PLAIN_ROLE), RoleId::new(MOD_ROLE)];

    let perms = guild_permissions(&roles, &facts);

    assert!(perms.manage_guild());
    assert!(perms.send_messages());
    assert!(!perms.administrator());
}

#[test]
fn unknown_roles_are_ignored() {
    let facts = facts(Permissions::empty());

    // A role the guild lookup didn't return (e.g. created after the facts were
    // cached) must not be assumed privileged.
    let perms = guild_permissions(&[RoleId::new(9_999)], &facts);

    assert!(perms.is_empty());
}

// The settings row is the only thing that arms the trap; a guild with no row
// must never match a channel.
#[test]
fn a_guild_with_no_row_has_the_honeypot_disabled() {
    let row = HoneypotSettingsRow::empty(as_i64(GUILD));

    assert_eq!(row.channel_id, None);
    assert!(!row.exempt_admins);
    assert_eq!(row.exempt_role_id, None);
    assert_eq!(HoneypotSettingsRow::TABLE, "honeypot_settings");
}
