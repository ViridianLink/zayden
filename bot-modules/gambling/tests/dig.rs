//! Regression tests for gambling DS-11 — the `/dig` half of the
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite class.
//!
//! DS-11 was that `/dig` read `gambling` + `gambling_mine`, mutated both in
//! memory across two awaits (`Dispatch::fire`, then the mine payout and
//! `done_work()`), and persisted the result **absolutely** —
//! `coins = EXCLUDED.coins, … stamina = EXCLUDED.stamina` and
//! `coal = EXCLUDED.coal, …`. Two digs in the same tick therefore spent one
//! stamina between them and the later write erased the earlier dig's ore.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord interaction,
//! for which this crate has no harness (audit CC-6), and the stamina guard
//! itself lives in SQL. What is testable in-process is what the fix introduces:
//! `DigDelta` — the *change* a dig made, applied atomically at write time — and
//! `MinePayout`, which carries the `mine_activity` the hourly accrual was
//! computed from so the commit can refuse to pay it twice.

use gambling::{DigDelta, DigRow, MinePayout};
use serenity::all::UserId;

const USER: UserId = UserId::new(1);

/// A dig's ore is a set of increments, not a post-image.
///
/// Two digs in the same tick each roll their own ore. Applying both deltas to
/// the live row keeps both; the absolute write kept only the later one.
#[test]
fn concurrent_digs_both_keep_their_ore() {
    let mut before = DigRow::new(USER);
    before.coal = 100;

    // First dig rolls 12 coal; the second, reading the same pre-image, rolls 5.
    let mut first = DigRow::new(USER);
    first.coal = before.coal + 12;
    let mut second = DigRow::new(USER);
    second.coal = before.coal + 5;

    let first_delta = DigDelta::between(&before, &first);
    let second_delta = DigDelta::between(&before, &second);

    assert_eq!(first_delta.coal, 12);
    assert_eq!(second_delta.coal, 5);

    // Fixed behaviour: both increments land on whatever the row holds.
    assert_eq!(before.coal + first_delta.coal + second_delta.coal, 117);

    // The absolute write the finding describes: each dig wrote `pre-image + own
    // roll`, so the later writer's total is all that survived.
    assert_ne!(second.coal, 117, "absolute write drops the first dig's 12 coal");
    assert_eq!(second.coal, 105);
}

/// Every ore column travels in the delta, so a dig that hits several of them at
/// once cannot lose one to a concurrent write of the same row.
#[test]
fn every_ore_column_is_an_increment() {
    let before = DigRow::new(USER);

    let mut after = DigRow::new(USER);
    after.coal = 9;
    after.iron = 4;
    after.gold = 2;
    after.redstone = 24;
    after.lapis = 18;
    after.diamonds = 1;
    after.emeralds = 3;

    let delta = DigDelta::between(&before, &after);

    assert_eq!(
        (
            delta.coal,
            delta.iron,
            delta.gold,
            delta.redstone,
            delta.lapis,
            delta.diamonds,
            delta.emeralds
        ),
        (9, 4, 2, 24, 18, 1, 3)
    );
}

/// The delta is the change between the row as read and the row after the dig's
/// in-memory mutations, so a credit that lands in that window survives.
#[test]
fn concurrent_credit_survives_the_dig_write() {
    let mut before = DigRow::new(USER);
    before.coins = 1_000;

    // A goal reward credited during `Dispatch::fire`.
    let mut after = DigRow::new(USER);
    after.coins = before.coins + 300;

    let delta = DigDelta::between(&before, &after);
    assert_eq!(delta.coins, 300);

    // Another command credited +500 between the read and the write.
    let current_in_db = 1_500;

    assert_eq!(current_in_db + delta.coins, 1_800);
    assert_ne!(after.coins, 1_800, "absolute overwrite would clobber the +500");
    assert_eq!(after.coins, 1_300, "…losing exactly the concurrent credit");
}

/// The hourly mine accrual is **not** part of the dig's delta.
///
/// It is time-based, so making it additive alongside the ore would let two digs
/// in the same tick each collect the same accrued hours — trading the finding's
/// lost update for a mint. It travels separately, tagged with the
/// `mine_activity` it was computed from, and the commit credits it only when
/// that stamp is still the live one.
#[test]
fn mine_payout_is_tagged_with_the_stamp_it_was_computed_from() {
    let read_at: jiff::Timestamp = "2026-07-28T00:00:00Z".parse().unwrap();
    let payout = MinePayout::new(2_400, read_at);

    assert_eq!(payout.coins, 2_400);
    assert_eq!(payout.since, read_at);
    assert!(
        payout.collected_at > payout.since,
        "the new stamp must advance past the one the accrual was measured from"
    );

    // A second dig in the same tick reads the same stamp and computes the same
    // accrual — the two are indistinguishable by value, which is why the commit
    // discriminates on the watermark they were measured from rather than on the
    // amount. It cannot discriminate on `collected_at`: that is a wall clock
    // truncated to TIMESTAMPTZ microseconds, so two digs in the same tick can
    // propose the *same* one. This used to assert they differed, and failed
    // whenever the clock did not advance between the two constructions; the
    // matching `RETURNING mine_activity = $collected_at` claim check inherited
    // the same weakness and paid the accrual to both racers. `commit_dig` now
    // guards the swap with `WHERE gambling_mine.mine_activity = $since`.
    let racing = MinePayout::new(2_400, read_at);
    assert_eq!(racing.since, payout.since, "both digs key on the same watermark");
    assert_eq!(racing.coins, payout.coins, "…and compute the same accrual");
    assert!(
        racing.collected_at > racing.since,
        "the loser's stamp is still a forward step, it just never lands"
    );
}

/// A payout of zero (no miners, or under an hour accrued) is still a valid
/// collection — it advances the stamp without crediting anything.
#[test]
fn zero_payout_still_carries_a_stamp() {
    let read_at = jiff::Timestamp::now();
    let payout = MinePayout::new(0, read_at);

    assert_eq!(payout.coins, 0);
    assert_eq!(payout.since, read_at);
}

/// A dig that finds nothing and triggers no rewards produces a no-op delta, so
/// the write cannot rewrite the row with stale values.
#[test]
fn empty_dig_yields_a_zero_delta() {
    let mut before = DigRow::new(USER);
    before.coins = 1_000;
    before.gems = 4;
    before.coal = 12;

    let delta = DigDelta::between(&before, &before);

    assert_eq!(delta, DigDelta::default());
    assert_eq!(delta.coins, 0);
    assert_eq!(delta.gems, 0);
    assert_eq!(delta.coal, 0);
}
