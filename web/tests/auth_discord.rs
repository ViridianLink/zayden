//! Guild ownership of submitted ids, and the bot's channel permissions
//! resolved the way Discord resolves them.

use topcoat::context::CxTestBuilder;
use twilight_model::channel::permission_overwrite::{
    PermissionOverwrite,
    PermissionOverwriteType,
};
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use web::auth::{
    ANNOUNCE_PERMISSIONS,
    GuildIds,
    channel_permissions,
    first_foreign,
};

const GUILD_IDS: [&str; 3] = ["100", "200", "300"];

#[test]
fn ids_the_guild_lists_pass() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 300]), None);
}

#[test]
fn nothing_submitted_passes() {
    assert_eq!(first_foreign(GUILD_IDS, &[]), None);
}

#[test]
fn an_id_from_another_guild_is_reported() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 999, 200]), Some(999));
}

#[test]
fn every_id_is_checked_not_just_the_first() {
    assert_eq!(first_foreign(GUILD_IDS, &[100, 200, 400]), Some(400));
}

#[test]
fn a_guild_with_no_listing_owns_nothing() {
    assert_eq!(first_foreign([], &[100]), Some(100));
}

/// No snowflake is negative, so a negative id must not slip through by
/// wrapping onto a listed one.
#[test]
fn a_negative_id_is_foreign() {
    let wrapped = (-1_i64).cast_unsigned().to_string();

    assert_eq!(first_foreign([wrapped.as_str()], &[-1]), Some(-1));
}

/// The context has no Discord client, so success proves no lookup ran.
#[tokio::test]
async fn an_empty_submission_needs_no_discord_lookup() {
    let cx = CxTestBuilder::new().build();

    assert!(
        GuildIds::default().channel(None).role(None).ensure_in(&cx, 1).await.is_ok()
    );
}

#[tokio::test]
async fn a_submission_without_a_discord_client_fails() {
    let cx = CxTestBuilder::new().build();

    let error =
        GuildIds::default().channel(Some(5)).ensure_in(&cx, 1).await.unwrap_err();

    assert_eq!(error.to_string(), "missing Discord client");
}

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

const fn bot_role_can_send() -> PermissionOverwrite {
    overwrite(
        BOT_ROLE,
        PermissionOverwriteType::Role,
        Permissions::SEND_MESSAGES,
        Permissions::empty(),
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
    assert!(can_announce(&[BOT_ROLE], &[
        everyone_cannot_send(),
        bot_role_can_send()
    ]));
}

#[test]
fn a_member_deny_overrides_a_role_allow() {
    let bot_cannot_send = overwrite(
        BOT,
        PermissionOverwriteType::Member,
        Permissions::empty(),
        Permissions::SEND_MESSAGES,
    );

    assert!(!can_announce(&[BOT_ROLE], &[bot_role_can_send(), bot_cannot_send]));
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
