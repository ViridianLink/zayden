//! Regression tests for `gold-star` —
//! [CC-6](../../../design-docs/audits/_cross-cutting.md) (the crate shipped zero
//! tests) covering the invariants [DS-1](../../../design-docs/audits/gold-star.md)
//! installed.
//!
//! DS-1 was that `/give_star` read the author and target rows, mutated them in
//! memory, and wrote both back with **absolute** `save_row` upserts. Concurrent
//! gives clobbered each other in both directions: stars minted (two gives, one
//! debit), stars lost (two credits, one landing), and the 24h free-star cap
//! bypassed. The fix made it one transaction: `FOR UPDATE` on the author, a
//! free-star arm that bumps `last_free_star`, a paid arm guarded by
//! `WHERE number_of_stars >= 1` with a `rows_affected` check, and an **atomic**
//! `+ 1` credit to the target.
//!
//! Every one of those lives in SQL ([`manager.rs:50-120`]), so unlike the other
//! crates' offline tests these need a live Postgres. They are the workspace's
//! first `#[sqlx::test]` suite: each test gets its own migrated database, so
//! `DATABASE_URL` must point at a server the test runner may create databases
//! on. Run them against a throwaway server, not a live one.
//!
//! [`manager.rs:50-120`]: ../src/manager.rs
//!
//! **Mutation coverage** (each guard removed in turn, suite re-run, then
//! reverted — this is the fails-before evidence, since the code under test was
//! already fixed by DS-1):
//!
//! | Guard removed | Result |
//! |---|---|
//! | `(last_free_star + INTERVAL '24 hours') <= now()` flipped to `>=` | 4 of 5 fail |
//! | `last_free_star = now()` bump on the free arm (the cap bypass) | `free_star_is_free_and_closes_its_window` fails |
//! | atomic credit → absolute `EXCLUDED.number_of_stars` (the DS-1 shape) | `concurrent_gives_to_one_target_both_land` fails with `[1, 1]` vs `[1, 2]` |
//! | `WHERE number_of_stars >= 1` **alone** | nothing fails — see the note on the paid test |
//! | all three overdraft guards together | 3 of 5 fail |
//!
//! **Note on the `users` foreign key:** `gold_stars.id` is
//! `REFERENCES users (id)`. [DS-2](../../../design-docs/audits/gold-star.md) was
//! that `give_star` never inserted a `users` row — `levels` and `family` both do
//! (`INSERT INTO users … ON CONFLICT DO NOTHING`) before touching their tables —
//! so a give whose author or target had no `users` row failed with a `23503`
//! foreign-key violation, surfaced as an opaque `GoldStarError::Sqlx`. The
//! fixture seeds `users` for the actors above so the tests below exercise the
//! star logic; the two `unseen_*` tests cover DS-2 itself with actors the
//! fixture deliberately omits.

use gold_star::{GoldStarError, GoldStarRow};
use serenity::all::UserId;
use sqlx::PgPool;

/// Free star spent (`last_free_star = now()`), holds 2 stars.
const PAID: UserId = UserId::new(100);
/// Free star spent, holds nothing — cannot give at all.
const EMPTY: UserId = UserId::new(200);
/// Free star window reopened (`now() - 25 hours`), holds nothing.
const EXPIRED_WINDOW: UserId = UserId::new(300);
/// Free star spent, holds 1 star.
const SECOND_PAID: UserId = UserId::new(400);
/// Has a `users` row but no `gold_stars` row yet.
const TARGET: UserId = UserId::new(900);
/// Deliberately absent from the fixture — no `users` row, no `gold_stars` row.
/// A member who has never sent a message, so levels never accrued one for them.
const UNSEEN_AUTHOR: UserId = UserId::new(500);
/// Deliberately absent from the fixture, as [`UNSEEN_AUTHOR`].
const UNSEEN_TARGET: UserId = UserId::new(950);

const FIXTURE: &str = "gold_stars";

/// `get_row`, with the query error unwrapped. Written as a macro rather than a
/// helper fn because `clippy.toml`'s `allow-expect-in-tests` only covers code
/// inside a `#[test]` item, and a free fn in a test binary is not one.
macro_rules! row {
    ($pool:expr, $user:expr) => {
        GoldStarRow::get_row(&$pool, $user).await.expect("get_row failed")
    };
}

/// The 24h free star: available once the window has passed, and **not** paid
/// for out of the star balance. Spending it closes the window immediately.
///
/// Catches: flipping the `(last_free_star + INTERVAL '24 hours') <= now()`
/// comparison, dropping the `last_free_star = now()` bump (which would make the
/// free star infinitely reusable — the DS-1 cap bypass), and debiting a star on
/// the free arm.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn free_star_is_free_and_closes_its_window(pool: PgPool) {
    let stars = GoldStarRow::give_star(
        &pool,
        EXPIRED_WINDOW,
        "expired-window-author",
        TARGET,
        "target",
    )
    .await
    .expect("the 25h-old window has reopened, so the give is free");
    assert_eq!(stars, 1, "the target's new star count is returned");

    let author = row!(pool, EXPIRED_WINDOW).expect("author row exists");
    assert_eq!(
        author.number_of_stars, 0,
        "the free star must not be paid for out of the balance"
    );
    assert_eq!(author.given_stars, 1, "the give is still counted");

    // The window is now closed and the author owns nothing, so the second give
    // has nothing to spend.
    let err = GoldStarRow::give_star(
        &pool,
        EXPIRED_WINDOW,
        "expired-window-author",
        TARGET,
        "target",
    )
    .await
    .expect_err("the free star was just spent; there is no second one");
    let GoldStarError::NoStars(next_free_star) = err else {
        panic!("expected NoStars, got {err:?}");
    };

    let in_24h = jiff::Timestamp::now().as_second() + 24 * 60 * 60;
    assert!(
        (next_free_star - in_24h).abs() <= 60,
        "the error carries the next free star at ~+24h, got {next_free_star} vs {in_24h}"
    );

    let target = row!(pool, TARGET).expect("target row exists");
    assert_eq!(
        target.number_of_stars, 1,
        "the refused give must not credit the target"
    );
}

/// The paid arm debits exactly one star per give and refuses at zero.
///
/// Catches: removing the overdraft guards — the balance then goes negative and
/// the target is credited anyway, which is the DS-1 mint.
///
/// Note what this cannot catch on its own: the three guards (the app-layer
/// `number_of_stars < 1` check, the SQL `WHERE number_of_stars >= 1` floor, and
/// the `rows_affected() != 1` assertion) are **mutually redundant in a single
/// process**, because the `FOR UPDATE` read already excludes a concurrent
/// balance change. Removing the SQL floor alone leaves every assertion here
/// green; the floor earns its keep only if the lock is ever weakened. All three
/// must go before this test fails — verified.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn paid_gives_debit_one_each_and_stop_at_zero(pool: PgPool) {
    for expected_target_stars in 1..=2 {
        let stars =
            GoldStarRow::give_star(&pool, PAID, "paid-author", TARGET, "target")
                .await
                .expect("the author holds stars");
        assert_eq!(stars, expected_target_stars);
    }

    let author = row!(pool, PAID).expect("author row exists");
    assert_eq!(author.number_of_stars, 0, "two gives, two stars debited");
    assert_eq!(author.given_stars, 2);

    let err = GoldStarRow::give_star(&pool, PAID, "paid-author", TARGET, "target")
        .await
        .expect_err("balance is zero and the free star is spent");
    assert!(matches!(err, GoldStarError::NoStars(_)), "got {err:?}");

    let author = row!(pool, PAID).expect("author row exists");
    assert_eq!(
        author.number_of_stars, 0,
        "the refused give must not overdraw the balance"
    );
    assert_eq!(author.given_stars, 2, "…and must not count as given");

    let target = row!(pool, TARGET).expect("target row exists");
    assert_eq!(target.number_of_stars, 2, "only the two paid gives landed");
    assert_eq!(target.received_stars, 2);
}

/// A refused give commits nothing at all — not even the author row the
/// transaction touches on its way in.
///
/// Catches: crediting the target before checking the author's balance, or
/// committing the transaction on the error path.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn a_refused_give_credits_nothing(pool: PgPool) {
    let err = GoldStarRow::give_star(&pool, EMPTY, "empty-author", TARGET, "target")
        .await
        .expect_err("the author holds nothing and has spent its free star");
    assert!(matches!(err, GoldStarError::NoStars(_)), "got {err:?}");

    assert!(
        row!(pool, TARGET).is_none(),
        "no star row may be minted for the target by a refused give"
    );

    let author = row!(pool, EMPTY).expect("author row exists");
    assert_eq!(author.given_stars, 0, "the refused give is not counted");
    assert_eq!(author.number_of_stars, 0);
}

/// The DS-1 scenario proper: two authors give to the **same** target at the
/// same time. Both credits must land.
///
/// Catches: reverting the credit to an absolute write
/// (`number_of_stars = EXCLUDED.number_of_stars`), under which both
/// transactions write `1` and one star vanishes.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn concurrent_gives_to_one_target_both_land(pool: PgPool) {
    let (first, second) = tokio::join!(
        GoldStarRow::give_star(&pool, PAID, "paid-author", TARGET, "target"),
        GoldStarRow::give_star(
            &pool,
            SECOND_PAID,
            "second-paid-author",
            TARGET,
            "target"
        )
    );

    let mut returned = [first.expect("first give"), second.expect("second give")];
    returned.sort_unstable();
    assert_eq!(
        returned,
        [1, 2],
        "the two gives observe consecutive star counts, never the same one"
    );

    let target = row!(pool, TARGET).expect("target row exists");
    assert_eq!(target.number_of_stars, 2, "both credits survive");
    assert_eq!(target.received_stars, 2);

    let paid = row!(pool, PAID).expect("author row exists");
    assert_eq!(paid.number_of_stars, 1, "started with 2, gave 1");
    let second_paid = row!(pool, SECOND_PAID).expect("author row exists");
    assert_eq!(second_paid.number_of_stars, 0, "started with 1, gave 1");
}

/// DS-2: an author with no `users` row can still give. This is the reachable
/// case — a member who has never sent a message has no `users` row, because
/// `levels` is what normally creates one on XP accrual.
///
/// Catches: dropping the author's `INSERT INTO users … ON CONFLICT DO NOTHING`,
/// which makes the `gold_stars` insert fail the `gold_stars.id REFERENCES
/// users (id)` foreign key with SQLSTATE `23503` — surfaced to the member as an
/// opaque `GoldStarError::Sqlx` with no user-facing message.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn an_unseen_author_can_give(pool: PgPool) {
    let stars = GoldStarRow::give_star(
        &pool,
        UNSEEN_AUTHOR,
        "unseen-author",
        TARGET,
        "target",
    )
    .await
    .expect("an author with no users row must not hit a foreign-key violation");
    assert_eq!(stars, 1, "the target is credited");

    // A brand-new author row starts at `last_free_star = to_timestamp(0)`, so
    // the give took the free arm rather than an empty balance.
    let author =
        row!(pool, UNSEEN_AUTHOR).expect("the author's star row was created");
    assert_eq!(author.given_stars, 1);
    assert_eq!(author.number_of_stars, 0, "the free star costs nothing");
}

/// DS-2, target side: a target with no `users` row can still be given to.
///
/// Catches: dropping the target's `INSERT INTO users … ON CONFLICT DO NOTHING`.
/// The author here is seeded, so only the credit's foreign key is under test.
#[sqlx::test(migrations = "../../migrations", fixtures("gold_stars"))]
async fn an_unseen_target_can_be_given_to(pool: PgPool) {
    let stars = GoldStarRow::give_star(
        &pool,
        PAID,
        "paid-author",
        UNSEEN_TARGET,
        "unseen-target",
    )
    .await
    .expect("a target with no users row must not hit a foreign-key violation");
    assert_eq!(stars, 1);

    let target =
        row!(pool, UNSEEN_TARGET).expect("the target's star row was created");
    assert_eq!(target.received_stars, 1);

    let author = row!(pool, PAID).expect("author row exists");
    assert_eq!(author.number_of_stars, 1, "started with 2, gave 1");
}

/// Guards the fixture itself: `FIXTURE` names the file the tests load, so a
/// rename that silently drops the seed data fails here rather than as a
/// confusing absent-row assertion elsewhere.
#[test]
fn fixture_is_the_one_the_tests_load() {
    assert!(
        std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/gold_stars.sql"
        ))
        .exists(),
        "missing tests/fixtures/{FIXTURE}.sql"
    );
}
