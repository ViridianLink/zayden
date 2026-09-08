//! Which quiet threads pick up the stale tag, and when they put it down.
//!
//! The rule that is easiest to get wrong is the one auto-close already makes:
//! a ticket nobody on the support team has answered is the team's backlog, not
//! an abandoned post, and it never goes stale for the team's silence.

use sqlx::PgPool;
use ticket::idle::ThreadActivity;
use ticket::idle::stale::{claim_cleared, claim_due};
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
        .expect("stale sweep")
        .into_iter()
        .map(|row| row.thread_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn cleared(pool: &PgPool) -> Vec<i64> {
    let mut ids = claim_cleared(pool, 40)
        .await
        .expect("stale clear sweep")
        .into_iter()
        .map(|row| row.thread_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn only_threads_the_poster_has_left_quiet_go_stale(pool: PgPool) {
    assert_eq!(claimed(&pool).await, vec![10, 11]);
}

/// The staff-side exemption, in one assertion. Threads 12 and 13 have waited a
/// month on the support team; neither is the poster's fault.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_thread_waiting_on_the_support_team_never_goes_stale(pool: PgPool) {
    let claimed = claimed(&pool).await;

    assert!(!claimed.contains(&12));
    assert!(!claimed.contains(&13));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_fresh_thread_and_a_closed_one_are_left_alone(pool: PgPool) {
    let claimed = claimed(&pool).await;

    assert!(!claimed.contains(&14));
    assert!(!claimed.contains(&15));
}

/// The claim stamps `staled_at` in the same statement, so a second sweeper -
/// or the next tick five minutes later - does not tag the same thread twice.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn the_claim_stamps_the_row_and_does_not_repeat(pool: PgPool) {
    assert_eq!(claimed(&pool).await, vec![10, 11]);
    assert_eq!(claimed(&pool).await, Vec::<i64>::new());
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_guild_needs_the_feature_on_and_a_tag_to_apply(pool: PgPool) {
    let claimed = claimed(&pool).await;

    // 20: no tag configured. 30: stale tagging switched off.
    assert!(!claimed.contains(&20));
    assert!(!claimed.contains(&30));
}

/// The sweep carries the guild's forum settings so the tagger does not have to
/// load them per thread.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn the_sweep_reports_the_forum_settings(pool: PgPool) {
    let due = claim_due(&pool, 40).await.expect("stale sweep");
    let row = due.iter().find(|r| r.thread_id == 10).expect("thread 10");

    assert_eq!(row.support_channel(), Some(ChannelId::new(500)));
    assert_eq!(row.stale_tag(), Some(ForumTagId::new(800)));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn the_batch_limit_is_respected(pool: PgPool) {
    assert_eq!(claim_due(&pool, 1).await.expect("stale sweep").len(), 1);
}

/// Thread 17 was tagged and then posted in, which is the whole trigger for
/// taking the tag off again.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_reply_after_the_tag_gives_the_thread_back(pool: PgPool) {
    assert_eq!(cleared(&pool).await, vec![17]);
}

/// A thread still sitting stale is not cleared just for being stale.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_thread_that_stayed_quiet_keeps_its_tag(pool: PgPool) {
    assert!(!cleared(&pool).await.contains(&16));
}

/// The clear claim wipes `staled_at`, so the same thread does not come round
/// again on the next tick - and can go stale afresh if the poster stops
/// answering a second time.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn clearing_happens_once(pool: PgPool) {
    assert_eq!(cleared(&pool).await, vec![17]);
    assert_eq!(cleared(&pool).await, Vec::<i64>::new());
}

/// `track` deliberately leaves `staled_at` standing so the sweep still sees a
/// thread that needs untagging; the reply only moves `since` past it.
#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_tracked_reply_queues_the_tag_for_removal(pool: PgPool) {
    claim_due(&pool, 40).await.expect("stale sweep");

    ThreadActivity::track(&pool, thread(10), OP, &[]).await.expect("track");

    assert!(cleared(&pool).await.contains(&10));
}

#[sqlx::test(migrations = "../../migrations", fixtures("support_stale"))]
async fn a_helper_reply_also_queues_the_tag_for_removal(pool: PgPool) {
    claim_due(&pool, 40).await.expect("stale sweep");

    ThreadActivity::track(&pool, thread(11), HELPER, &[SUPPORT])
        .await
        .expect("track");

    assert!(cleared(&pool).await.contains(&11));
}
