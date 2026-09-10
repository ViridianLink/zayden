//! Which rows each sweep claims, and — more importantly — which it leaves
//! alone. A sweep that over-claims suspends a server somebody has paid for.

use hosting::{store, sweep};
use sqlx::PgPool;

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
fn ids(rows: Result<Vec<sweep::Claimed>, hosting::HostingError>) -> Vec<i64> {
    let mut ids: Vec<i64> =
        rows.expect("sweep").into_iter().map(|row| row.id).collect();
    ids.sort_unstable();
    ids
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn expiry_claims_only_priced_rows_past_their_window(pool: PgPool) {
    let claimed = ids(sweep::claim_expired(&pool, 3, 40).await);

    assert_eq!(
        claimed,
        vec![1, 2],
        "expected the lapsed paid month and the lapsed priced trial only"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn expiry_leaves_included_plans_for_the_conversion_sweep(pool: PgPool) {
    let claimed = ids(sweep::claim_expired(&pool, 3, 40).await);

    assert!(
        !claimed.contains(&3),
        "an included plan has no window of its own to run out"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn expiry_is_idempotent_across_passes(pool: PgPool) {
    let first = ids(sweep::claim_expired(&pool, 3, 40).await);
    let second = ids(sweep::claim_expired(&pool, 3, 40).await);

    assert_eq!(first, vec![1, 2]);
    assert!(second.is_empty(), "a claimed row must not be claimed twice");
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn expiry_sets_the_grace_clock(pool: PgPool) {
    sweep::claim_expired(&pool, 3, 40).await.ok();

    let row = store::get(&pool, 1).await.ok();
    let state = row.map(|r| r.state);

    assert_eq!(state.as_deref(), Some("suspended"));
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn finished_included_trials_convert_rather_than_suspend(pool: PgPool) {
    let claimed = ids(sweep::convert_included_trials(&pool, 40).await);

    assert_eq!(claimed, vec![3]);

    let row = store::get(&pool, 3).await.ok();
    let (state, billing) =
        row.map_or((None, None), |r| (Some(r.state), Some(r.billing_source)));

    assert_eq!(state.as_deref(), Some("active"));
    assert_eq!(billing.as_deref(), Some("entitlement"));
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn deletion_claims_only_rows_past_their_grace(pool: PgPool) {
    let claimed = ids(sweep::claim_deletions(&pool, 40).await);

    assert_eq!(claimed, vec![5]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn a_claimed_deletion_leases_rather_than_releases(pool: PgPool) {
    let first = ids(sweep::claim_deletions(&pool, 40).await);
    let second = ids(sweep::claim_deletions(&pool, 40).await);

    assert_eq!(first, vec![5]);
    assert!(
        second.is_empty(),
        "the lease must hold long enough for the panel call to finish"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn reminders_fire_once_per_window(pool: PgPool) {
    let first = ids(sweep::claim_reminders(&pool, 2, 40).await);
    let second = ids(sweep::claim_reminders(&pool, 2, 40).await);

    assert_eq!(first, vec![7], "row 8 was already reminded for this window");
    assert!(second.is_empty(), "a reminder must never be sent twice");
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn trials_never_get_an_expiry_reminder(pool: PgPool) {
    // Row 11's trial ends in an hour, well inside a two-day reminder window.
    // Reminders are a subscription courtesy; firing one here would DM the owner
    // before the server had finished installing.
    let claimed = ids(sweep::claim_reminders(&pool, 2, 40).await);

    assert!(!claimed.contains(&11), "a trial has no renewal to be reminded of");
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn a_lapsed_trial_gets_no_grace_but_a_lapsed_subscription_does(
    pool: PgPool,
) {
    let expired = ids(sweep::claim_expired(&pool, 3, 40).await);
    assert_eq!(expired, vec![1, 2], "row 1 is a subscription, row 2 a trial");

    let deletable = ids(sweep::claim_deletions(&pool, 40).await);

    assert!(
        deletable.contains(&2),
        "a trial nobody paid for must not quietly earn the subscription grace \
         period on top of its trial time"
    );
    assert!(
        !deletable.contains(&1),
        "a lapsed subscription keeps its three-day grace"
    );
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn revalidation_sees_only_subscription_billed_rows(pool: PgPool) {
    let rows = ids(sweep::entitlement_backed(&pool, 40).await);

    assert_eq!(rows, vec![9]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn deleted_rows_are_invisible_to_every_sweep(pool: PgPool) {
    let expired = ids(sweep::claim_expired(&pool, 3, 40).await);
    let deletions = ids(sweep::claim_deletions(&pool, 40).await);
    let reminders = ids(sweep::claim_reminders(&pool, 2, 40).await);
    let converted = ids(sweep::convert_included_trials(&pool, 40).await);

    for claimed in [expired, deletions, reminders, converted] {
        assert!(!claimed.contains(&10), "row 10 is already deleted");
    }
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn the_per_user_cap_counts_live_rows_only(pool: PgPool) {
    let owner = serenity::all::UserId::new(3);
    let count = store::live_count(&pool, owner).await.ok();

    // Rows 5, 6 and 9 are live; row 10 is deleted.
    assert_eq!(count, Some(3));
}

#[sqlx::test(migrations = "../../migrations", fixtures("hosting"))]
async fn global_usage_excludes_deleted_memory(pool: PgPool) {
    let usage = store::global_usage(&pool).await.ok();
    let (servers, _memory) = usage.unwrap_or((0, 0));

    assert_eq!(servers, 10, "ten of the eleven fixture rows are live");
}
