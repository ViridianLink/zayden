use deadlock::db::announce::targets;
use sqlx::PgPool;

const ZAYDEN: i64 = 1000;

#[sqlx::test(migrations = "../../migrations", fixtures("announce_targets"))]
async fn targets_exclude_disabled_absent_and_custom_served_guilds(pool: PgPool) {
    let rows = targets(&pool, ZAYDEN).await.unwrap();

    let mut guilds: Vec<i64> = rows.iter().map(|row| row.guild_id).collect();
    guilds.sort_unstable();

    assert_eq!(guilds, vec![1, 2]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("announce_targets"))]
async fn another_application_only_sees_the_guilds_it_is_in(pool: PgPool) {
    let rows = targets(&pool, 3000).await.unwrap();

    assert!(rows.is_empty());
}
