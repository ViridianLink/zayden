//! `lookup_session` is the only read of `web_sessions`, so both of its paths
//! carry weight: the miss that fills the cache, and the hit that skips Postgres
//! entirely. The hit is what turns a settings-page render's thirteen session
//! SELECTs into one.
#![cfg(feature = "ssr")]

use std::time::Duration;

use dashboard::server::auth::{SessionCache, SessionIdentity, lookup_session};
use moka::future::Cache;
use sqlx::PgPool;

fn cache() -> SessionCache {
    Cache::builder().max_capacity(16).time_to_live(Duration::from_secs(60)).build()
}

#[sqlx::test(migrations = "../migrations", fixtures("web_session"))]
async fn a_miss_reads_the_row_and_records_it(pool: PgPool) {
    let cache = cache();

    let identity = lookup_session(Some(&cache), &pool, "live-token")
        .await
        .expect("the session query succeeds")
        .expect("the fixture session is live");

    assert_eq!(identity.user_id, 41);
    assert_eq!(identity.access_token, "live-access-token");

    let cached = cache.get("live-token").await.expect("the miss filled the cache");
    assert_eq!(cached.user_id, 41);
    assert_eq!(cached.access_token, "live-access-token");
}

/// The token deliberately has no row: only the cache can answer, so a value
/// coming back proves the database was not consulted.
#[sqlx::test(migrations = "../migrations", fixtures("web_session"))]
async fn a_hit_answers_without_the_database(pool: PgPool) {
    let cache = cache();
    cache
        .insert("cached-only-token".to_owned(), SessionIdentity {
            user_id: 77,
            access_token: "cached-only-access-token".to_owned(),
        })
        .await;

    let identity = lookup_session(Some(&cache), &pool, "cached-only-token")
        .await
        .expect("a hit never queries")
        .expect("the cached session answers");

    assert_eq!(identity.user_id, 77);
    assert_eq!(identity.access_token, "cached-only-access-token");
}

#[sqlx::test(migrations = "../migrations", fixtures("web_session"))]
async fn an_unknown_token_is_not_a_session(pool: PgPool) {
    let cache = cache();

    let identity = lookup_session(Some(&cache), &pool, "no-such-token")
        .await
        .expect("the session query succeeds");

    assert!(identity.is_none());
    assert!(cache.get("no-such-token").await.is_none());
}

/// An expired row is a miss, and must not be cached as if it were live.
#[sqlx::test(migrations = "../migrations", fixtures("web_session"))]
async fn an_expired_session_is_not_revived_by_the_cache(pool: PgPool) {
    let cache = cache();

    let identity = lookup_session(Some(&cache), &pool, "expired-token")
        .await
        .expect("the session query succeeds");

    assert!(identity.is_none());
    assert!(cache.get("expired-token").await.is_none());
}

/// Callers outside a Leptos context — the Axum Patreon handler — pass no cache
/// and must still resolve.
#[sqlx::test(migrations = "../migrations", fixtures("web_session"))]
async fn the_lookup_works_with_no_cache_at_all(pool: PgPool) {
    let identity = lookup_session(None, &pool, "live-token")
        .await
        .expect("the session query succeeds")
        .expect("the fixture session is live");

    assert_eq!(identity.user_id, 41);
}
