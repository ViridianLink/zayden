//! Building Discord's command-permission arrays.
//!
//! Discord only accepts a whole-array overwrite, so every writer is a
//! read-modify-write and every writer can therefore destroy settings it does
//! not own. Two pages write to the same array — the modules page owns the
//! `@everyone` switch, a feature's own page owns the channel entries — and an
//! admin may have added user overrides in Discord that belong to neither.

use dashboard::server::command_permissions::{
    all_channels,
    channel_allowlist,
    everyone,
    everyone_denied,
    with_channel_allowlist,
    with_everyone_denied,
};
use twilight_model::application::command::permissions::{
    CommandPermission,
    CommandPermissionType,
};
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, GuildMarker};

/// `channel_allowlist` returns `Vec<Id<ChannelMarker>>`; this names the empty
/// case so the assertions can compare against it without spelling the type out.
const NO_CHANNELS: [Id<ChannelMarker>; 0] = [];

const GUILD: Id<GuildMarker> = Id::new(900);
const GENERAL: Id<ChannelMarker> = Id::new(11);
const OFF_TOPIC: Id<ChannelMarker> = Id::new(12);

/// Written out by hand rather than via `all_channels`, so the sentinel test
/// below compares the implementation against Discord's documented encoding
/// instead of against itself.
const ALL_CHANNELS: Id<ChannelMarker> = Id::new(899);

const fn deny_all_channels() -> CommandPermission {
    CommandPermission {
        id: CommandPermissionType::Channel(ALL_CHANNELS),
        permission: false,
    }
}

const fn allow_channel(id: Id<ChannelMarker>) -> CommandPermission {
    CommandPermission { id: CommandPermissionType::Channel(id), permission: true }
}

const fn deny_everyone() -> CommandPermission {
    CommandPermission {
        id: CommandPermissionType::Role(everyone(GUILD)),
        permission: false,
    }
}

const fn allow_user(id: u64) -> CommandPermission {
    CommandPermission {
        id: CommandPermissionType::User(Id::new(id)),
        permission: true,
    }
}

// region: the all-channels sentinel

#[test]
fn the_all_channels_sentinel_is_the_guild_id_minus_one() {
    // Off by one in either direction and the deny lands on a real channel, or
    // on nothing, and the restriction silently stops applying.
    assert_eq!(all_channels(GUILD), Some(ALL_CHANNELS));
}

// endregion

// region: reading

#[test]
fn no_overwrites_means_no_restriction() {
    assert_eq!(channel_allowlist(GUILD, &[]), NO_CHANNELS);
    assert!(!everyone_denied(GUILD, &[]));
}

#[test]
fn allows_without_the_deny_all_are_not_a_restriction() {
    // Discord treats a bare allow as a no-op: the command already ran there.
    // Reporting it as an allowlist would show a restriction that is not on.
    assert_eq!(channel_allowlist(GUILD, &[allow_channel(GENERAL)]), NO_CHANNELS);
}

#[test]
fn the_allowlist_is_the_allows_alongside_the_deny_all() {
    let perms =
        [deny_all_channels(), allow_channel(GENERAL), allow_channel(OFF_TOPIC)];

    assert_eq!(channel_allowlist(GUILD, &perms), vec![GENERAL, OFF_TOPIC]);
}

#[test]
fn the_everyone_switch_is_read_independently_of_channels() {
    let perms = [deny_everyone(), deny_all_channels(), allow_channel(GENERAL)];

    assert!(everyone_denied(GUILD, &perms));
    assert_eq!(channel_allowlist(GUILD, &perms), vec![GENERAL]);
}

// endregion

// region: writing channels

#[test]
fn setting_an_allowlist_writes_the_deny_all_plus_one_allow_each() {
    let out = with_channel_allowlist(GUILD, &[], &[GENERAL, OFF_TOPIC]);

    assert!(
        out.contains(&deny_all_channels()),
        "without the deny-all, the \
         allows do nothing and the command stays open everywhere"
    );
    assert!(out.contains(&allow_channel(GENERAL)));
    assert!(out.contains(&allow_channel(OFF_TOPIC)));
    assert_eq!(out.len(), 3);
}

#[test]
fn an_empty_allowlist_clears_the_restriction_entirely() {
    let current = [deny_all_channels(), allow_channel(GENERAL)];
    let out = with_channel_allowlist(GUILD, &current, &[]);

    assert!(
        out.is_empty(),
        "dropping the last channel must also drop the deny-all, or /good \
         would be left runnable nowhere"
    );
}

#[test]
fn writing_channels_preserves_the_module_switch_and_user_overrides() {
    let current = [deny_everyone(), allow_user(7), deny_all_channels()];
    let out = with_channel_allowlist(GUILD, &current, &[GENERAL]);

    assert!(
        out.contains(&deny_everyone()),
        "editing channels must not silently re-enable a disabled module"
    );
    assert!(out.contains(&allow_user(7)));
    assert_eq!(channel_allowlist(GUILD, &out), vec![GENERAL]);
}

#[test]
fn the_sentinel_cannot_be_added_as_an_ordinary_channel() {
    let out = with_channel_allowlist(GUILD, &[], &[ALL_CHANNELS, GENERAL]);

    assert_eq!(
        out.iter().filter(|p| **p == deny_all_channels()).count(),
        1,
        "an allow on the sentinel would contradict the deny on the same id"
    );
    assert_eq!(channel_allowlist(GUILD, &out), vec![GENERAL]);
}

// endregion

// region: writing the module switch

#[test]
fn disabling_a_module_preserves_the_channel_restriction() {
    let current = [deny_all_channels(), allow_channel(GENERAL)];
    let out = with_everyone_denied(GUILD, &current, true);

    assert!(everyone_denied(GUILD, &out));
    assert_eq!(
        channel_allowlist(GUILD, &out),
        vec![GENERAL],
        "toggling a module off must not wipe where its commands are allowed"
    );
}

#[test]
fn re_enabling_a_module_preserves_the_channel_restriction() {
    let current = [deny_everyone(), deny_all_channels(), allow_channel(GENERAL)];
    let out = with_everyone_denied(GUILD, &current, false);

    assert!(!everyone_denied(GUILD, &out));
    assert_eq!(channel_allowlist(GUILD, &out), vec![GENERAL]);
}

#[test]
fn the_everyone_entry_is_never_duplicated() {
    let out = with_everyone_denied(GUILD, &[deny_everyone()], true);

    assert_eq!(out, vec![deny_everyone()]);
}

// endregion
