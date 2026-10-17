mod common;

use std::time::Duration;

use common::{Reply, Requests, api, spawn_mock};
use deadlock::DeadlockError;
use deadlock_api::{DeadlockApiError, Priority};
use sqlx::PgPool;

const RANKS: &str = include_str!("fixtures/ranks.json");

fn count(requests: &Requests) -> usize {
    requests.lock().map_or(0, |recorded| recorded.len())
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_repeated_call_is_served_from_memory_then_from_postgres(pool: PgPool) {
    let mock = spawn_mock(Reply::ok(RANKS)).await.unwrap();
    let api_one = api(&pool, &mock.base).unwrap();

    let first = api_one.ranks(Priority::Interactive).await.unwrap();
    api_one.ranks(Priority::Interactive).await.unwrap();

    assert!(!first.stale);
    assert_eq!(first.value.len(), 12);
    assert_eq!(count(&mock.requests), 1);

    let api_two = api(&pool, &mock.base).unwrap();
    let again = api_two.ranks(Priority::Interactive).await.unwrap();

    assert!(!again.stale);
    assert_eq!(again.value.len(), 12);
    assert_eq!(count(&mock.requests), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_expired_row_is_served_stale_when_the_api_is_busy(pool: PgPool) {
    let mock = spawn_mock(Reply::ok(RANKS)).await.unwrap();
    api(&pool, &mock.base).unwrap().ranks(Priority::Interactive).await.unwrap();

    sqlx::query!(
        "UPDATE deadlock_api_cache SET expires_at = now() - interval '1 hour'"
    )
    .execute(&pool)
    .await
    .unwrap();

    mock.serve("/v1/assets/ranks", Reply {
        status: 503,
        headers: vec![("retry-after", "7")],
        body: String::new(),
    });

    let stale =
        api(&pool, &mock.base).unwrap().ranks(Priority::Interactive).await.unwrap();

    assert!(stale.stale);
    assert_eq!(stale.value.len(), 12);
    assert_eq!(count(&mock.requests), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_busy_api_without_a_cached_row_is_an_error(pool: PgPool) {
    let mock = spawn_mock(Reply {
        status: 503,
        headers: vec![("retry-after", "7")],
        body: String::new(),
    })
    .await
    .unwrap();

    let err = api(&pool, &mock.base)
        .unwrap()
        .ranks(Priority::Interactive)
        .await
        .unwrap_err();

    assert!(
        matches!(
            err,
            DeadlockError::Api(DeadlockApiError::Busy { retry_after })
                if retry_after == Duration::from_secs(7)
        ),
        "{err:?}"
    );
}
