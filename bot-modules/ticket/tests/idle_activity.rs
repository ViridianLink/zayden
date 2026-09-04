//! The ball-in-court state machine, against a real database.
//!
//! Two cases here are the reason the transition lives in SQL rather than in
//! Rust: a staff member opening their own ticket must not be nudged for their
//! own silence, and a bystander wandering through a public forum post must not
//! reset the clock on somebody else's conversation.

use sqlx::PgPool;
use ticket::idle::sweep::claim_due;
use ticket::idle::{Ball, ThreadActivity};
use ticket::{GuildId, RoleId, ThreadId, UserId};

const GUILD: GuildId = GuildId::new(1);
const OP: UserId = UserId::new(1000);
const HELPER: UserId = UserId::new(2000);
const BYSTANDER: UserId = UserId::new(3000);
const SUPPORT: RoleId = RoleId::new(100);
const UNRELATED: RoleId = RoleId::new(999);

const fn thread(id: u64) -> ThreadId {
    ThreadId::new(id)
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn claimed(pool: &PgPool) -> Vec<i64> {
    let mut ids = claim_due(pool, 40)
        .await
        .expect("sweep")
        .into_iter()
        .map(|row| row.thread_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn only_stalled_threads_in_enabled_guilds_come_due(pool: PgPool) {
    assert_eq!(claimed(&pool).await, vec![10, 11, 12]);
}

/// The claim and the read are one statement, so a second sweeper - or a second
/// tick before the first finished - finds nothing left to ping.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_claimed_thread_is_not_claimed_twice(pool: PgPool) {
    assert_eq!(claimed(&pool).await.len(), 3);
    assert_eq!(claimed(&pool).await, Vec::<i64>::new());
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn the_sweep_reports_the_ball_and_the_roles(pool: PgPool) {
    let due = claim_due(&pool, 40).await.expect("sweep");

    let unanswered = due.iter().find(|r| r.thread_id == 10).expect("thread 10");
    assert_eq!(unanswered.ball(), Ball::Helper);
    assert_eq!(unanswered.helper(), None);
    assert_eq!(unanswered.support_roles(), vec![SUPPORT]);

    let waiting_on_op = due.iter().find(|r| r.thread_id == 11).expect("thread 11");
    assert_eq!(waiting_on_op.ball(), Ball::Op);
    assert_eq!(waiting_on_op.helper(), Some(HELPER));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_helper_reply_moves_the_ball_to_the_poster(pool: PgPool) {
    ThreadActivity::track(&pool, thread(10), HELPER, &[SUPPORT])
        .await
        .expect("track");

    let row = ThreadActivity::active(&pool, thread(10))
        .await
        .expect("active")
        .expect("row");

    assert_eq!(row.ball(), Ball::Op);
    assert_eq!(row.helper_id, Some(2000));
    assert!(claimed(&pool).await.iter().all(|id| *id != 10));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_poster_reply_moves_the_ball_back_to_the_helpers(pool: PgPool) {
    ThreadActivity::track(&pool, thread(11), OP, &[]).await.expect("track");

    let row = ThreadActivity::active(&pool, thread(11))
        .await
        .expect("active")
        .expect("row");

    assert_eq!(row.ball(), Ball::Helper);
    assert!(claimed(&pool).await.iter().all(|id| *id != 11));
}

/// The regression the SQL `WHERE` tail exists for: anyone can post in a public
/// forum ticket, and a passer-by is neither side of the conversation.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_bystander_does_not_move_the_ball(pool: PgPool) {
    ThreadActivity::track(&pool, thread(10), BYSTANDER, &[UNRELATED])
        .await
        .expect("track");

    assert!(claimed(&pool).await.contains(&10));
}

/// A staff member's own ticket. The `op_id` test wins over the role test, so
/// they are never nudged to answer themselves.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn the_poster_holding_a_support_role_is_still_the_poster(pool: PgPool) {
    ThreadActivity::track(&pool, thread(10), OP, &[SUPPORT]).await.expect("track");

    let row = ThreadActivity::active(&pool, thread(10))
        .await
        .expect("active")
        .expect("row");

    assert_eq!(row.ball(), Ball::Helper);
    assert_eq!(row.helper_id, None);
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn the_poster_does_not_overwrite_the_helper(pool: PgPool) {
    ThreadActivity::track(&pool, thread(12), OP, &[]).await.expect("track");

    let row = ThreadActivity::active(&pool, thread(12))
        .await
        .expect("active")
        .expect("row");

    assert_eq!(row.helper_id, Some(2000));
}

/// A reply re-arms a thread that has already had its one nudge.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_reply_clears_a_spent_nudge(pool: PgPool) {
    assert!(!claimed(&pool).await.contains(&14));

    ThreadActivity::track(&pool, thread(14), HELPER, &[SUPPORT])
        .await
        .expect("track");

    // Re-armed, but the clock restarted, so it is not due again yet.
    assert!(!claimed(&pool).await.contains(&14));
}

/// A solved or closed ticket reads as absent, which is what makes the buttons
/// refuse a second press.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn a_paused_thread_is_not_active_and_ignores_replies(pool: PgPool) {
    assert!(
        ThreadActivity::active(&pool, thread(15)).await.expect("active").is_none()
    );

    ThreadActivity::track(&pool, thread(15), HELPER, &[SUPPORT])
        .await
        .expect("track");

    assert!(
        ThreadActivity::active(&pool, thread(15)).await.expect("active").is_none()
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn pausing_and_resuming_survives_the_helper(pool: PgPool) {
    ThreadActivity::pause(&pool, thread(12)).await.expect("pause");
    assert!(
        ThreadActivity::active(&pool, thread(12)).await.expect("active").is_none()
    );

    ThreadActivity::resume(&pool, thread(12)).await.expect("resume");

    let row = ThreadActivity::active(&pool, thread(12))
        .await
        .expect("active")
        .expect("row");

    assert_eq!(row.ball(), Ball::Helper);
    assert_eq!(row.helper_id, Some(2000));
    // Reopening restarts the clock rather than firing immediately.
    assert!(!claimed(&pool).await.contains(&12));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle"))]
async fn insert_is_idempotent_and_delete_is_final(pool: PgPool) {
    ThreadActivity::insert(&pool, GUILD, thread(30), OP).await.expect("insert");
    ThreadActivity::insert(&pool, GUILD, thread(30), BYSTANDER)
        .await
        .expect("insert again");

    let row = ThreadActivity::active(&pool, thread(30))
        .await
        .expect("active")
        .expect("row");
    assert_eq!(row.op(), OP);

    ThreadActivity::delete(&pool, thread(30)).await.expect("delete");
    assert!(
        ThreadActivity::active(&pool, thread(30)).await.expect("active").is_none()
    );
}
