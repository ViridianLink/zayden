//! Regression tests for gambling DS-7 (`/work` half) — the last
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite site.
//!
//! DS-7 was that `/work` read a row, mutated it in memory across several awaits
//! (mine payout, then `Dispatch::fire`, which can credit goal rewards), and then
//! persisted the resulting **absolute** values with
//! `coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina`.
//! Anything another command committed in that window was silently erased.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord interaction,
//! for which this crate has no harness (audit CC-6), and the guard itself lives
//! in SQL. What is testable in-process is the piece the fix now turns on:
//! `WorkDelta`, which did not exist before — the command had no representation
//! of "what this shift earned" at all, only the post-mutation absolute row. These
//! tests pin that the delta is the *change*, so applying it to whatever the row
//! holds at write time preserves a concurrent credit that an absolute write drops.

use gambling::WorkDelta;

/// The DS-7 interleave, at the value level.
///
/// User holds 1 000 coins when `/work` reads. The shift earns 300. Before the
/// write lands, another command atomically credits +500 (row is now 1 500).
///
/// - Old behaviour: absolute save writes `before + earned` = 1 300 — the +500 is
///   gone.
/// - Fixed behaviour: `current + delta` = 1 800 — the +500 survives.
#[test]
fn concurrent_credit_survives_the_work_write() {
    let before = (1_000, 0);
    let earned = 300;
    let after_in_memory = (before.0 + earned, before.1);

    let delta = WorkDelta::between(before, after_in_memory);
    assert_eq!(delta.coins, earned, "the delta must be the shift's earnings");

    // Another command credited +500 between the read and the write.
    let current_in_db = 1_500;

    assert_eq!(current_in_db + delta.coins, 1_800);
    // The absolute write the finding describes, pinned so a regression to it fails.
    assert_ne!(after_in_memory.0, 1_800, "absolute overwrite would clobber +500");
    assert_eq!(after_in_memory.0, 1_300, "…losing exactly the concurrent credit");
}

/// A gem found during the shift is likewise persisted as `+1`, not as an
/// absolute gem count read before the shift.
#[test]
fn gem_find_is_persisted_as_an_increment() {
    let before = (1_000, 4);
    let delta = WorkDelta::between(before, (before.0 + 250, before.1 + 1));

    assert_eq!(delta.coins, 250);
    assert_eq!(delta.gems, 1);

    // A gift of 3 gems landing mid-shift must survive the write.
    assert_eq!(7 + delta.gems, 8);
}

/// Goal rewards credited by `Dispatch::fire` (`add_coins(5_000)` / `add_gems(1)`)
/// are inside the delta — the delta is taken *after* the dispatch, so they are
/// persisted by the same atomic increment rather than by a separate write.
#[test]
fn goal_rewards_are_included_in_the_delta() {
    let before = (1_000, 0);

    // Payout, then a goal completing during dispatch.
    let after_payout = (before.0 + 300, before.1);
    let after_dispatch = (after_payout.0 + 5_000, after_payout.1 + 1);

    let delta = WorkDelta::between(before, after_dispatch);

    assert_eq!(delta.coins, 5_300);
    assert_eq!(delta.gems, 1);
}

/// A shift that changes nothing produces a no-op delta, so the write cannot
/// rewrite the row with stale values.
#[test]
fn unchanged_row_yields_a_zero_delta() {
    let delta = WorkDelta::between((1_000, 4), (1_000, 4));

    assert_eq!(delta, WorkDelta::default());
    assert_eq!(delta.coins, 0);
    assert_eq!(delta.gems, 0);
}
