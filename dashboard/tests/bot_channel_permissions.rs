//! The announcement-channel check resolves the bot's permissions the way
//! Discord does. The case that motivated it: a read-only channel where
//! `@everyone` is denied Send Messages and the bot's roles have no overwrite.
#![cfg(feature = "ssr")]

use dashboard::server::discord::{ANNOUNCE_PERMISSIONS, channel_permissions};
use twilight_model::channel::permission_overwrite::{
    PermissionOverwrite,
    PermissionOverwriteType,
};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;

const GUILD: u64 = 100;
const BOT: u64 = 200;
const BOT_ROLE: u64 = 300;
const OTHER_ROLE: u64 = 400;

fn roles() -> Vec<(u64, Permissions)> {
    vec![
        (GUILD, Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES),
        (BOT_ROLE, Permissions::EMBED_LINKS),
        (OTHER_ROLE, Permissions::ADMINISTRATOR),
    ]
}

const fn overwrite(
    id: u64,
    kind: PermissionOverwriteType,
    allow: Permissions,
    deny: Permissions,
) -> PermissionOverwrite {
    PermissionOverwrite { allow, deny, id: Id::new(id), kind }
}

const fn everyone_cannot_send() -> PermissionOverwrite {
    overwrite(
        GUILD,
        PermissionOverwriteType::Role,
        Permissions::empty(),
        Permissions::SEND_MESSAGES,
    )
}

fn can_announce(member_roles: &[u64], overwrites: &[PermissionOverwrite]) -> bool {
    channel_permissions(GUILD, BOT, member_roles, &roles(), overwrites)
        .contains(ANNOUNCE_PERMISSIONS)
}

#[test]
fn an_open_channel_allows_posting() {
    assert!(can_announce(&[BOT_ROLE], &[]));
}

#[test]
fn an_everyone_deny_blocks_the_bot() {
    assert!(!can_announce(&[BOT_ROLE], &[everyone_cannot_send()]));
}

#[test]
fn a_role_allow_overrides_the_everyone_deny() {
    let bot_role_can_send = overwrite(
        BOT_ROLE,
        PermissionOverwriteType::Role,
        Permissions::SEND_MESSAGES,
        Permissions::empty(),
    );

    assert!(can_announce(&[BOT_ROLE], &[everyone_cannot_send(), bot_role_can_send]));
}

#[test]
fn a_member_deny_overrides_a_role_allow() {
    let bot_role_can_send = overwrite(
        BOT_ROLE,
        PermissionOverwriteType::Role,
        Permissions::SEND_MESSAGES,
        Permissions::empty(),
    );
    let bot_cannot_send = overwrite(
        BOT,
        PermissionOverwriteType::Member,
        Permissions::empty(),
        Permissions::SEND_MESSAGES,
    );

    assert!(!can_announce(&[BOT_ROLE], &[bot_role_can_send, bot_cannot_send]));
}

#[test]
fn an_overwrite_for_a_role_the_bot_lacks_is_ignored() {
    let other_role_can_send = overwrite(
        OTHER_ROLE + 1,
        PermissionOverwriteType::Role,
        Permissions::SEND_MESSAGES,
        Permissions::empty(),
    );

    assert!(!can_announce(&[BOT_ROLE], &[
        everyone_cannot_send(),
        other_role_can_send
    ]));
}

#[test]
fn administrator_bypasses_overwrites() {
    assert!(can_announce(&[OTHER_ROLE], &[everyone_cannot_send()]));
}
