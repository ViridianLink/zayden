use deadlock::db::guild_posts::claim;
use deadlock::db::modules::enabled_guilds;
use deadlock::db::summary_channel::targets_for;
use sqlx::PgPool;

const ZAYDEN: i64 = 1000;

#[sqlx::test(migrations = "../../migrations", fixtures("summary_targets"))]
async fn targets_need_a_channel_an_opt_in_presence_and_the_module(pool: PgPool) {
    let targets = targets_for(&pool, 10, ZAYDEN).await.unwrap();

    let guilds: Vec<(i64, i64)> =
        targets.iter().map(|target| (target.guild_id, target.channel_id)).collect();

    assert_eq!(guilds, vec![(1, 101), (2, 102)]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("summary_targets"))]
async fn targets_are_per_member(pool: PgPool) {
    let targets = targets_for(&pool, 11, ZAYDEN).await.unwrap();

    let guilds: Vec<i64> = targets.iter().map(|target| target.guild_id).collect();

    assert_eq!(guilds, vec![1]);
    assert!(targets_for(&pool, 12, ZAYDEN).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("summary_targets"))]
async fn another_application_only_sees_the_guilds_it_is_in(pool: PgPool) {
    let targets = targets_for(&pool, 10, 3000).await.unwrap();

    assert!(targets.is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("summary_targets"))]
async fn a_match_is_claimed_once_per_guild(pool: PgPool) {
    assert!(claim(&pool, 1, 555).await.unwrap());
    assert!(!claim(&pool, 1, 555).await.unwrap());
    assert!(claim(&pool, 2, 555).await.unwrap());
    assert!(claim(&pool, 1, 556).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations", fixtures("summary_targets"))]
async fn enabled_guilds_filter_by_presence_and_module(pool: PgPool) {
    let enabled = enabled_guilds(&pool, &[1, 2, 3, 5, 6, 99], ZAYDEN).await.unwrap();

    assert_eq!(enabled, vec![1, 2]);
    assert_eq!(enabled_guilds(&pool, &[], ZAYDEN).await.unwrap(), Vec::<i64>::new());
}
