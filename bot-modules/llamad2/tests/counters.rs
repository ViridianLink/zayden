//! Regression tests for the `llamad2` novelty counters —
//! [CC-6](../../../design-docs/audits/_cross-cutting.md) (the crate shipped zero
//! tests) covering the invariant [finding
//! #1](../../../design-docs/audits/llamad2.md) installed.
//!
//! Finding #1 was that both counters were flat JSON files read, parsed, mutated
//! and rewritten with blocking `std::fs` **on the async message path**. The fix
//! (`f2cf893d`) moved them to `llamad2_counters` and replaced the
//! read-modify-write with one server-side upsert
//! (`count = llamad2_counters.count + 1`, per
//! [CC-9](../../../design-docs/audits/_cross-cutting.md)) — but landed with no
//! test, because the workspace had no live-`PgPool` harness at the time. It has
//! one now (`gold-star`, `9a7b8795`), so these are that fix's missing net.
//!
//! The whole of the behaviour under test is the one SQL statement in
//! [`Counter::bump`], so like `gold-star`'s suite these need a live Postgres:
//! each test gets its own migrated database, and `DATABASE_URL` must point at a
//! server the test runner may **create databases on**. Use a throwaway server,
//! never a live one — see [`CLAUDE.md`](../../../CLAUDE.md).
//!
//! [`Counter::bump`]: ../src/counter.rs
//!
//! There is no fixture: `llamad2_counters` starts empty, which is itself the
//! first case under test (the flat-file version errored on a freshly-created
//! empty file — `serde_json::from_str("")` — so the very first fail of a fresh
//! deploy was dropped).
//!
//! **Mutation coverage** (each property broken in turn, suite re-run, then
//! reverted — this is the fails-before evidence, since the code under test was
//! already fixed by finding #1):
//!
//! | Mutation | Result |
//! |---|---|
//! | `DO UPDATE SET count = llamad2_counters.count + 1` → `= EXCLUDED.count` (absolute write, the pre-fix shape) | 2 of 3 DB tests fail — the concurrency one with `[1, 1]` vs `[1, 2]` |
//! | `VALUES ($1, 1)` → `VALUES ($1, 0)` | all 3 DB tests fail |
//! | `ON CONFLICT (name)` arm dropped (plain `INSERT`) | 2 of 3 DB tests fail — `23505` unique violation |
//! | `Counter::COUNTING_FAILS` renamed | `counter_names_are_the_persisted_ones` fails |

use llamad2::Counter;
use sqlx::PgPool;

const MIGRATIONS: &str = "../../migrations";

/// A counter that does not exist yet starts at `1`, not `0` and not an error.
///
/// This is the empty-file bug the DB move fixed: the first-ever counting fail
/// on a fresh deploy used to hit `serde_json::from_str("")` and be dropped.
#[sqlx::test(migrations = "../../migrations")]
async fn bump_creates_a_missing_counter_at_one(pool: PgPool) {
    let count = Counter::bump(&pool, Counter::COUNTING_FAILS)
        .await
        .expect("bumping an absent counter must create it");

    assert_eq!(count, 1, "the first bump reports one, not zero");
}

/// Each bump returns the post-increment value, and the two counters do not
/// share a row.
#[sqlx::test(migrations = "../../migrations")]
async fn bump_increments_and_returns_the_new_value(pool: PgPool) {
    for expected in 1..=3 {
        let count = Counter::bump(&pool, Counter::COUNTING_FAILS)
            .await
            .expect("bump failed");
        assert_eq!(count, expected, "bump {expected} must return {expected}");
    }

    let dumb = Counter::bump(&pool, Counter::DUMB_COUNT).await.expect("bump failed");
    assert_eq!(
        dumb, 1,
        "the counters are keyed by name; /goof must not inherit the counting fails"
    );

    let fails =
        Counter::bump(&pool, Counter::COUNTING_FAILS).await.expect("bump failed");
    assert_eq!(fails, 4, "…and must not be reset by the other counter");
}

/// The reason the counters moved to the DB: two messages in the same tick must
/// both be counted.
///
/// Discord dispatches each message on its own tokio task, so the flat-file
/// read-modify-write lost one of any two concurrent fails. Catches reverting
/// the upsert to an absolute write (`SET count = EXCLUDED.count`), under which
/// both callers observe `1` and the counter ends at `1` instead of `2`.
#[sqlx::test(migrations = "../../migrations")]
async fn bumps_are_atomic_under_concurrency(pool: PgPool) {
    let (first, second) = tokio::join!(
        Counter::bump(&pool, Counter::COUNTING_FAILS),
        Counter::bump(&pool, Counter::COUNTING_FAILS)
    );

    let mut observed = [first.expect("first bump"), second.expect("second bump")];
    observed.sort_unstable();
    assert_eq!(
        observed,
        [1, 2],
        "concurrent bumps must observe distinct consecutive values"
    );

    // Reading the total back through one more bump: 3 proves the two above both
    // landed, without needing a reader the crate does not have.
    let total =
        Counter::bump(&pool, Counter::COUNTING_FAILS).await.expect("bump failed");
    assert_eq!(total, 3, "neither concurrent bump was lost");
}

/// The counter names are the table's primary key, so renaming one silently
/// orphans a live counter and restarts it from zero in production.
///
/// Offline on purpose: reading the rows back would need a `query!` in a test
/// binary, whose `.sqlx` entry the workspace's `cargo sqlx prepare` (no
/// `--all-targets`) does not generate, and a runtime `sqlx::query` would
/// reintroduce exactly the bypass
/// [CC-5](../../../design-docs/audits/_cross-cutting.md) closed. The names the
/// tests above write are these constants, so pinning them is what the assertion
/// is worth either way.
#[test]
fn counter_names_are_the_persisted_ones() {
    assert_eq!(Counter::COUNTING_FAILS, "counting_fails");
    assert_eq!(Counter::DUMB_COUNT, "dumb_count");
}

/// Guards the migration path the tests load, so a directory move fails here
/// rather than as an opaque "relation does not exist" in every test above.
#[test]
fn migrations_are_where_the_tests_look() {
    assert!(
        std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../migrations"
        ))
        .join("0016_llamad2_counters.up.sql")
        .exists(),
        "missing {MIGRATIONS}/0016_llamad2_counters.up.sql"
    );
}
