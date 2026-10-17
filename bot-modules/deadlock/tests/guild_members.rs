use deadlock::db::guild_members::{
    remove,
    set_share_summaries,
    set_squad,
    share_summaries,
    squad,
};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn leaving_resets_both_opt_ins(pool: PgPool) {
    set_share_summaries(&pool, 1, 10, true).await.unwrap();
    set_squad(&pool, 1, 10, true).await.unwrap();

    assert!(remove(&pool, 1, 10).await.unwrap());

    assert!(!share_summaries(&pool, 1, 10).await.unwrap());
    assert!(!squad(&pool, 1, 10).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn leaving_keeps_opt_ins_in_other_guilds(pool: PgPool) {
    set_share_summaries(&pool, 1, 10, true).await.unwrap();
    set_share_summaries(&pool, 2, 10, true).await.unwrap();

    remove(&pool, 1, 10).await.unwrap();

    assert!(share_summaries(&pool, 2, 10).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn leaving_without_opt_ins_is_a_no_op(pool: PgPool) {
    assert!(!remove(&pool, 1, 10).await.unwrap());
}
