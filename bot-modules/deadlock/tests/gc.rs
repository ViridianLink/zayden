use deadlock::gc::{GcOutcome, run};
use jiff::Timestamp;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations", fixtures("gc"))]
async fn expired_rows_are_pruned_and_recent_or_pinned_ones_survive(pool: PgPool) {
    let now: Timestamp = "2026-10-17T00:00:00Z".parse().unwrap();

    let outcome = run(&pool, now).await.unwrap();

    assert_eq!(outcome, GcOutcome {
        cache_rows: 1,
        rank_snapshots: 1,
        matches: 1,
        guild_posts: 1,
        briefings: 1,
        asset_snapshots: 2,
    });

    let keys = sqlx::query_scalar!("SELECT key FROM deadlock_api_cache")
        .fetch_all(&pool)
        .await
        .unwrap();
    let versions = sqlx::query_scalar!(
        "SELECT client_version FROM deadlock_asset_snapshots ORDER BY client_version"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(keys, vec!["expired-two-days"]);
    assert_eq!(versions, vec![2, 4, 5, 6, 7, 8]);
}
