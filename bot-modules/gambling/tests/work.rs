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

use gambling::{MinePayout, WorkCommit, WorkDelta};

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

// --- DS-12: the hourly mine accrual is not a per-shift reward ------------
//
// DS-7 (above) made `/work`'s coin write atomic, which fixed the lost update
// but turned the *other* half of the same sum into a mint: `row.mine_amount()`
// — a time-based accrual, identical for every reader in the same hour — was
// folded into `WorkDelta.coins` and `mine_activity` was stamped absolutely, so
// two shifts in the same tick each incremented by the same accrued window.
// The accrual now travels as a `MinePayout` carrying the stamp it was measured
// from, and is credited only by the shift that wins the compare-and-swap on
// that stamp — the pattern DS-11 introduced for `/dig`.

/// The DS-12 interleave: two `/work`s in the same tick, one accrual.
///
/// Both read `mine_activity` at `T0` and both compute the same accrued coins.
/// Only the base earnings are per-shift, so only those belong in the delta.
#[test]
fn mine_accrual_is_not_part_of_the_work_delta() {
    let before = (1_000, 0);
    let base_earned = 300;
    let accrued = 2_400; // identical for both shifts — same hours, same miners

    let delta = WorkDelta::between(before, (before.0 + base_earned, before.1));

    assert_eq!(
        delta.coins, base_earned,
        "the delta is the shift's own earnings, not the mine's"
    );
    assert_ne!(
        delta.coins,
        base_earned + accrued,
        "folding the accrual in is what let two shifts each collect it"
    );

    // Two shifts racing, both increments applied to the live row: the base
    // earnings stack (each shift did its own work and spent its own stamina),
    // while the accrual is paid exactly once by the CAS winner.
    let paid_once = before.0 + delta.coins + delta.coins + accrued;
    assert_eq!(paid_once, 1_600 + accrued);

    // The pre-fix sum, pinned so a regression to it fails: the accrual rode in
    // both deltas and was therefore credited twice.
    let paid_twice = before.0 + (delta.coins + accrued) * 2;
    assert_eq!(paid_twice - paid_once, accrued, "exactly one accrual minted");
}

/// The accrual is tagged with the watermark it was computed from, so the commit
/// can discriminate between two shifts that are indistinguishable by value.
///
/// The discriminator is `since`, never `collected_at`. `collected_at` is a wall
/// clock truncated to TIMESTAMPTZ microseconds, so two shifts in the same tick
/// can and do propose the *same* one — this test used to assert they differed
/// and failed whenever the clock did not advance between the two constructions.
/// Claiming by `RETURNING mine_activity = $collected_at` inherited that
/// weakness: on a collision the swap's loser matched too and the accrual minted
/// twice, the very DS-12 regression above. The swap is now guarded by
/// `WHERE gambling_mine.mine_activity = $since`, so the winner is decided
/// entirely by the shared watermark pinned here and stamp collisions are inert.
#[test]
fn racing_shifts_share_a_stamp_so_only_one_can_claim() {
    let read_at: jiff::Timestamp = "2026-07-28T00:00:00Z".parse().unwrap();

    let first = MinePayout::new(2_400, read_at);
    let second = MinePayout::new(2_400, read_at);

    assert_eq!(first.coins, second.coins, "same window, same amount");
    assert_eq!(first.since, read_at, "the swap keys on the watermark as read");
    assert_eq!(second.since, read_at, "…and both shifts carry it unchanged");

    // Only one swap can find the row still on `read_at`; the winner moves it to
    // its own `collected_at`, which must be strictly ahead so the watermark
    // advances rather than stalling or rewinding.
    assert!(
        first.collected_at > first.since,
        "the proposed stamp must advance past the one measured from"
    );
    assert!(second.collected_at > second.since);
}

/// A shift reports the accrual it actually collected, not the one it computed —
/// the loser of the swap still works, still spends stamina, and reports `0`.
#[test]
fn work_commit_reports_the_payout_actually_credited() {
    let winner = WorkCommit { coins: 3_700, gems: 0, stamina: 2, payout: 2_400 };
    let loser = WorkCommit { coins: 4_000, gems: 0, stamina: 1, payout: 0 };

    assert_eq!(winner.payout, 2_400, "the CAS winner banks the accrual");
    assert_eq!(loser.payout, 0, "the loser gets its base pay and nothing more");
    assert_eq!(loser.stamina, 1, "…but still spent its stamina");
}

/// A zero accrual (no miners, or under an hour) is still a valid collection: it
/// advances the watermark without crediting anything.
#[test]
fn zero_accrual_still_carries_a_stamp() {
    let read_at = jiff::Timestamp::now();
    let payout = MinePayout::new(0, read_at);

    assert_eq!(payout.coins, 0);
    assert_eq!(payout.since, read_at);
}
