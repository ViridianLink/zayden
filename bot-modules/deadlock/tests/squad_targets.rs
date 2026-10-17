use deadlock::db::squad_channel::{
    delete,
    leaderboard_guilds,
    select,
    set_leaderboard_message,
    targets_for,
    upsert,
};
use sqlx::PgPool;

const ZAYDEN: i64 = 1000;

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn targets_need_squad_membership_a_presence_and_the_module(pool: PgPool) {
    let targets = targets_for(&pool, 10, ZAYDEN).await.unwrap();

    let guilds: Vec<(i64, i64)> =
        targets.iter().map(|target| (target.guild_id, target.channel_id)).collect();

    assert_eq!(guilds, vec![(1, 101), (2, 102)]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn targets_are_per_member(pool: PgPool) {
    let targets = targets_for(&pool, 11, ZAYDEN).await.unwrap();

    let guilds: Vec<i64> = targets.iter().map(|target| target.guild_id).collect();

    assert_eq!(guilds, vec![1]);
    assert!(targets_for(&pool, 12, ZAYDEN).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn another_application_only_sees_the_guilds_it_is_in(pool: PgPool) {
    assert!(targets_for(&pool, 10, 3000).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn leaderboards_go_only_to_gated_guilds_with_the_toggle_on(pool: PgPool) {
    let guilds: Vec<i64> = leaderboard_guilds(&pool, ZAYDEN)
        .await
        .unwrap()
        .iter()
        .map(|row| row.guild_id)
        .collect();

    assert_eq!(guilds, vec![1, 4]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn changing_the_channel_forgets_the_stored_leaderboard_message(pool: PgPool) {
    set_leaderboard_message(&pool, 1, 555).await.unwrap();

    upsert(&pool, 1, 101, false).await.unwrap();
    let kept = select(&pool, 1).await.unwrap().unwrap();
    assert_eq!(kept.leaderboard_message_id, Some(555));
    assert!(!kept.leaderboard);

    upsert(&pool, 1, 999, true).await.unwrap();
    let moved = select(&pool, 1).await.unwrap().unwrap();
    assert_eq!(moved.channel_id, 999);
    assert_eq!(moved.leaderboard_message_id, None);
    assert!(moved.leaderboard);
}

#[sqlx::test(migrations = "../../migrations", fixtures("squad_targets"))]
async fn deleting_the_channel_reports_whether_it_existed(pool: PgPool) {
    assert!(delete(&pool, 1).await.unwrap());
    assert!(!delete(&pool, 1).await.unwrap());
    assert!(select(&pool, 1).await.unwrap().is_none());
}
