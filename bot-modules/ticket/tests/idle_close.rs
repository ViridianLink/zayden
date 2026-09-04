//! Which stalled threads the auto-close sweep is allowed to take.
//!
//! The load-bearing rule is the one that is easiest to get wrong: a ticket the
//! support team has not answered is never closed for the team's silence. Only
//! the poster going quiet after being asked ends a thread.

use sqlx::PgPool;
use ticket::idle::ThreadActivity;
use ticket::idle::close::claim_due;
use ticket::{ChannelId, ForumTagId, RoleId, ThreadId, UserId};

const OP: UserId = UserId::new(1000);
const HELPER: UserId = UserId::new(2000);
const SUPPORT: RoleId = RoleId::new(100);

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
        .expect("close sweep")
        .into_iter()
        .map(|row| row.thread_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn only_reminded_threads_the_poster_owes_come_due(pool: PgPool) {
    assert_eq!(claimed(&pool).await, vec![10, 11]);
}

/// The whole staff-side exemption, in one assertion. Threads 12 and 13 have sat
/// reminded for twenty days waiting on the support team; neither is closable.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn a_thread_waiting_on_the_support_team_is_never_closed(pool: PgPool) {
    let claimed = claimed(&pool).await;

    assert!(!claimed.contains(&12));
    assert!(!claimed.contains(&13));
}

/// The race the reference design needed a second API read to avoid: any tracked
/// reply clears `nudged_at`, so the row disqualifies itself before the sweep
/// ever looks at it.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn a_reply_after_the_reminder_cancels_the_close(pool: PgPool) {
    ThreadActivity::track(&pool, thread(10), OP, &[]).await.expect("track");

    assert_eq!(claimed(&pool).await, vec![11]);
}

/// A helper answering the poster's silence also cancels it - the ball moves
/// back to the poster with a fresh clock, not a spent reminder.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn a_helper_reply_after_the_reminder_cancels_the_close(pool: PgPool) {
    ThreadActivity::track(&pool, thread(11), HELPER, &[SUPPORT])
        .await
        .expect("track");

    assert_eq!(claimed(&pool).await, vec![10]);
}

/// The claim pauses the row in the same statement, so a second sweeper - or a
/// retry after Discord failed us - never closes the same thread twice.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn the_claim_pauses_the_row_and_does_not_repeat(pool: PgPool) {
    assert_eq!(claimed(&pool).await, vec![10, 11]);

    assert!(
        ThreadActivity::active(&pool, thread(10)).await.expect("active").is_none()
    );
    assert_eq!(claimed(&pool).await, Vec::<i64>::new());
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn a_guild_must_enable_both_reminders_and_closing(pool: PgPool) {
    let claimed = claimed(&pool).await;

    // 20: auto-close off. 30: reminders off, so nothing ever gets reminded.
    assert!(!claimed.contains(&20));
    assert!(!claimed.contains(&30));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn a_fresh_reminder_and_an_unsent_one_both_wait(pool: PgPool) {
    let claimed = claimed(&pool).await;

    assert!(!claimed.contains(&14));
    assert!(!claimed.contains(&15));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn an_already_closed_thread_is_left_alone(pool: PgPool) {
    assert!(!claimed(&pool).await.contains(&16));
}

/// The sweep carries the guild's forum settings so the closer does not have to
/// load them per thread.
#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn the_sweep_reports_the_forum_settings(pool: PgPool) {
    let due = claim_due(&pool, 40).await.expect("close sweep");
    let row = due.iter().find(|r| r.thread_id == 10).expect("thread 10");

    assert_eq!(row.op(), OP);
    assert_eq!(row.support_channel(), Some(ChannelId::new(500)));
    assert_eq!(row.closed_tag(), Some(ForumTagId::new(700)));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_idle_close"))]
async fn the_batch_limit_is_respected(pool: PgPool) {
    assert_eq!(claim_due(&pool, 1).await.expect("close sweep").len(), 1);
}
