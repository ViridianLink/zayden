//! Permission gate for the admin `/destiny2 builds refresh` subcommand.
//!
//! `refresh` drops the in-memory loadout cache and reloads from the database so
//! manual/website edits to the `destiny2_loadout*` tables take effect without a
//! restart. It is gated on Manage Server (matching the other admin subcommands),
//! so the gate must reject members lacking that permission and anyone whose
//! permissions could not be resolved (e.g. invoked outside a guild).

use destiny2::loadouts::is_privileged;
use serenity::all::Permissions;

#[test]
fn manage_guild_is_privileged() {
    assert!(is_privileged(Some(Permissions::MANAGE_GUILD)));
}

#[test]
fn administrator_with_manage_guild_is_privileged() {
    assert!(is_privileged(Some(
        Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD
    )));
}

#[test]
fn lacking_manage_guild_is_not_privileged() {
    assert!(!is_privileged(Some(
        Permissions::SEND_MESSAGES | Permissions::MANAGE_MESSAGES
    )));
    assert!(!is_privileged(Some(Permissions::empty())));
}

#[test]
fn unresolved_permissions_are_not_privileged() {
    assert!(!is_privileged(None));
}
