//! `lookup_session_user` is the only call of Discord's `GET /users/@me` after
//! login; its cache hit keeps the account menu from calling Discord on every
//! page. `account_name` is what the menu shows.

use std::time::Duration;

use moka::future::Cache;
use web::auth::{SessionUser, lookup_session_user};
use web::shell::account_name;
use web::state::{SessionIdentity, SessionUsersCache};

fn cache() -> SessionUsersCache {
    Cache::builder().max_capacity(16).time_to_live(Duration::from_mins(10)).build()
}

fn user() -> SessionUser {
    SessionUser { id: "41".to_owned(), name: "Oscar".to_owned(), avatar: None }
}

/// The token is unusable, so any user coming back proves Discord was not
/// consulted.
fn identity() -> SessionIdentity {
    SessionIdentity {
        user_id: 41,
        access_token: "not-a-real-access-token".to_owned(),
    }
}

#[tokio::test]
async fn repeated_lookups_answer_from_the_cache() {
    let cache = cache();
    cache.insert(41, user()).await;

    let first = lookup_session_user(Some(&cache), &identity()).await;
    let second = lookup_session_user(Some(&cache), &identity()).await;

    assert_eq!(first, Some(user()));
    assert_eq!(second, Some(user()));
    assert_eq!(cache.get(&41).await, Some(user()));
}

#[test]
fn the_menu_shows_the_user_name() {
    assert_eq!(account_name(Some(&user())), "Oscar");
}

#[test]
fn the_menu_falls_back_to_account_without_a_user() {
    assert_eq!(account_name(None), "Account");
}
