//! `lookup_user_guilds` is the only call of Discord's `GET /users/@me/guilds`,
//! and one settings-page render asks for it roughly eight times. The hit path
//! is what collapses those round-trips into one. `manages_guild` is the
//! predicate the server switcher's list and the per-guild authorization now
//! share, so it has to agree with what Discord actually grants.
#![cfg(feature = "ssr")]

use std::sync::Arc;
use std::time::Duration;

use dashboard::server::auth::{
    SessionIdentity,
    UserGuildsCache,
    lookup_user_guilds,
    manages_guild,
};
use moka::future::Cache;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;

fn cache() -> UserGuildsCache {
    Cache::builder().max_capacity(16).time_to_live(Duration::from_secs(60)).build()
}

fn guild(id: u64, permissions: Permissions) -> CurrentUserGuild {
    CurrentUserGuild {
        id: Id::new(id),
        name: format!("Guild {id}"),
        icon: None,
        owner: false,
        permissions,
        features: Vec::new(),
    }
}

/// The token is deliberately unusable, so any list coming back proves Discord
/// was not consulted.
fn identity() -> SessionIdentity {
    SessionIdentity {
        user_id: 41,
        access_token: "not-a-real-access-token".to_owned(),
    }
}

#[tokio::test]
async fn a_hit_answers_without_calling_discord() {
    let cache = cache();
    cache.insert(41, Arc::from([guild(7, Permissions::ADMINISTRATOR)])).await;

    let guilds = lookup_user_guilds(Some(&cache), &identity())
        .await
        .expect("the cached list answers");

    assert_eq!(guilds.len(), 1);
    assert_eq!(guilds[0].id.get(), 7);
}

/// Both consumers read this one payload: the switcher filters it, and
/// `guild_admin_for` looks one id up in it. They must see the same list.
#[tokio::test]
async fn one_cached_payload_serves_the_switcher_and_the_authorization() {
    let cache = cache();
    cache
        .insert(
            41,
            Arc::from([
                guild(7, Permissions::MANAGE_GUILD),
                guild(8, Permissions::SEND_MESSAGES),
            ]),
        )
        .await;

    let guilds = lookup_user_guilds(Some(&cache), &identity())
        .await
        .expect("the cached list answers");

    let manageable: Vec<u64> =
        guilds.iter().filter(|g| manages_guild(g)).map(|g| g.id.get()).collect();

    assert_eq!(manageable, vec![7]);
    assert!(guilds.iter().any(|g| g.id.get() == 8));
}

#[test]
fn administrator_manages_a_guild() {
    assert!(manages_guild(&guild(7, Permissions::ADMINISTRATOR)));
}

#[test]
fn manage_guild_alone_manages_a_guild() {
    assert!(manages_guild(&guild(7, Permissions::MANAGE_GUILD)));
}

#[test]
fn an_unrelated_permission_does_not_manage_a_guild() {
    assert!(!manages_guild(&guild(7, Permissions::SEND_MESSAGES)));
}

#[test]
fn no_permissions_do_not_manage_a_guild() {
    assert!(!manages_guild(&guild(7, Permissions::empty())));
}
