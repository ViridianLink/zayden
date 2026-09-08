//! `CommandState::enabled` is the fix for the audit's only P0 finding
//! (DATA-01): before it existed, a failed Discord read produced an empty
//! command map, and `known.is_empty() || …` short-circuited to `true`, so
//! every command-backed module rendered "Enabled" with total confidence.
//! The tri-state return exists so "unknown" can never again be reported as
//! "enabled".
#![cfg(feature = "ssr")]

use std::collections::HashMap;

use dashboard::server::command_permissions::everyone;
use dashboard::server::modules::CommandState;
use twilight_model::application::command::permissions::{
    CommandPermission,
    CommandPermissionType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{CommandMarker, GuildMarker, RoleMarker};

const GUILD: Id<GuildMarker> = Id::new(900);

const fn deny_everyone(guild_id: Id<GuildMarker>) -> CommandPermission {
    CommandPermission {
        id: CommandPermissionType::Role(everyone(guild_id)),
        permission: false,
    }
}

const fn deny_role(role_id: Id<RoleMarker>) -> CommandPermission {
    CommandPermission { id: CommandPermissionType::Role(role_id), permission: false }
}

fn name_to_id(pairs: &[(&str, u64)]) -> HashMap<String, Id<CommandMarker>> {
    pairs.iter().map(|(name, id)| ((*name).to_owned(), Id::new(*id))).collect()
}

fn permissions(
    pairs: &[(u64, Vec<CommandPermission>)],
) -> HashMap<Id<CommandMarker>, Vec<CommandPermission>> {
    pairs.iter().map(|(id, perms)| (Id::new(*id), perms.clone())).collect()
}

#[test]
fn every_registered_name_present_and_nothing_denied_is_enabled() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("music", 1)]),
        &permissions(&[(1, vec![])]),
    );

    assert_eq!(state.enabled(&["music"]), Some(true));
}

#[test]
fn a_single_command_module_denied_for_everyone_is_disabled() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("music", 1)]),
        &permissions(&[(1, vec![deny_everyone(GUILD)])]),
    );

    assert_eq!(state.enabled(&["music"]), Some(false));
}

#[test]
fn a_multi_command_module_with_one_allowed_command_is_enabled() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("blackjack", 1), ("coinflip", 2), ("roll", 3)]),
        &permissions(&[
            (1, vec![deny_everyone(GUILD)]),
            (2, vec![deny_everyone(GUILD)]),
            (3, vec![]),
        ]),
    );

    assert_eq!(state.enabled(&["blackjack", "coinflip", "roll"]), Some(true));
}

#[test]
fn a_multi_command_module_denied_on_every_command_is_disabled() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("blackjack", 1), ("coinflip", 2)]),
        &permissions(&[
            (1, vec![deny_everyone(GUILD)]),
            (2, vec![deny_everyone(GUILD)]),
        ]),
    );

    assert_eq!(state.enabled(&["blackjack", "coinflip"]), Some(false));
}

/// The DATA-01 regression guard: a non-empty `name_to_id` from other guilds'
/// commands must not be mistaken for this module's registration.
#[test]
fn none_of_the_modules_names_in_a_non_empty_map_is_unknown_not_enabled() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("other_module_command", 1)]),
        &permissions(&[]),
    );

    let result = state.enabled(&["music"]);

    assert_eq!(result, None);
    assert_ne!(result, Some(true), "unknown must never be reported as enabled");
}

#[test]
fn an_entirely_empty_command_map_is_unknown() {
    let state = CommandState::new(GUILD, name_to_id(&[]), &permissions(&[]));

    assert_eq!(state.enabled(&["music"]), None);
}

#[test]
fn one_registered_and_denied_name_out_of_two_is_disabled_not_unknown() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("family", 1)]),
        &permissions(&[(1, vec![deny_everyone(GUILD)])]),
    );

    assert_eq!(state.enabled(&["family", "unregistered_alias"]), Some(false));
}

#[test]
fn denying_a_role_that_is_not_everyone_does_not_count_as_denied() {
    let some_other_role: Id<RoleMarker> = Id::new(42);
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("music", 1)]),
        &permissions(&[(1, vec![deny_role(some_other_role)])]),
    );

    assert_eq!(state.enabled(&["music"]), Some(true));
}

#[test]
fn a_denial_on_a_different_modules_command_id_does_not_affect_this_module() {
    let state = CommandState::new(
        GUILD,
        name_to_id(&[("music", 1), ("palworld", 2)]),
        &permissions(&[(2, vec![deny_everyone(GUILD)])]),
    );

    assert_eq!(state.enabled(&["music"]), Some(true));
}
