//! Operator accounts bypass guild-level permission checks, so two details that
//! are invisible at compile time decide whether the bypass is safe and honest:
//! the exact string looked up in `web_user_roles`, and the one capability the
//! bypass cannot grant.

use dashboard::server::auth::{GuildAccess, WebRole};
use dashboard::ui::pages::operator_servers::parse_guild_id;

/// `web_user_roles.role` is plain `text` with no CHECK constraint, so a typo
/// here does not fail loudly -- it just silently matches no row and denies
/// access forever. The migration writes these literals; this pins them.
#[test]
fn web_roles_map_to_the_literals_stored_in_the_database() {
    assert_eq!(WebRole::Admin.as_str(), "admin");
    assert_eq!(WebRole::Operator.as_str(), "operator");
}

#[test]
fn the_operator_role_is_distinct_from_the_admin_role() {
    assert_ne!(WebRole::Admin.as_str(), WebRole::Operator.as_str());
}

/// Discord only accepts a command-permission overwrite from a bearer token
/// belonging to a member with Manage Server. An operator viewing a server they
/// have not joined has no such token, so this must stay false -- flipping it
/// would turn a clear refusal into a silent failure against Discord's API.
#[test]
fn only_member_access_may_write_command_permissions() {
    assert!(GuildAccess::Member.can_write_command_permissions());
    assert!(!GuildAccess::Operator.can_write_command_permissions());
}

#[test]
fn a_snowflake_is_accepted_as_a_server_id() {
    assert_eq!(parse_guild_id("98765432109876543"), Some(98_765_432_109_876_543));
    assert_eq!(
        parse_guild_id("  98765432109876543  "),
        Some(98_765_432_109_876_543)
    );
}

/// The "go to server ID" box is the only way to reach a server that is hard to
/// find by name, so it must reject anything that would navigate to a guild
/// page which cannot possibly load.
#[test]
fn non_snowflakes_are_rejected_as_server_ids() {
    assert_eq!(parse_guild_id(""), None);
    assert_eq!(parse_guild_id("   "), None);
    assert_eq!(parse_guild_id("0"), None);
    assert_eq!(parse_guild_id("not-an-id"), None);
    assert_eq!(parse_guild_id("123abc"), None);
    assert_eq!(parse_guild_id("-1"), None);
    // A pasted guild URL is not an id, even though it contains one.
    assert_eq!(parse_guild_id("https://discord.com/channels/123"), None);
    // Wider than u64.
    assert_eq!(parse_guild_id("99999999999999999999999"), None);
}
