//! Regression tests for gambling DS-9 — the `/shop buy`
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite site.
//!
//! DS-9 was that `/shop buy` read the whole `ShopRow` (coins, gems, the three
//! crafted currencies, and the nine mine columns), deducted the cost **in
//! memory**, ran `Dispatch::fire` (which can credit goal rewards) across an
//! await, and then persisted the resulting **absolute** values with
//! `coins = EXCLUDED.coins, gems = EXCLUDED.gems` plus twelve absolute
//! `gambling_mine` columns. Two `/shop buy` invocations in the same tick both
//! read the pre-image and the second write clobbered the first: two items
//! delivered, one charged.
//!
//! As with [`work.rs`](work.rs) (DS-7), the end-to-end command path needs a live
//! `PgPool` plus a Discord interaction, for which this crate has no harness
//! (audit CC-6), and the guard itself lives in SQL. What is testable in-process
//! is the value-level piece the fix turns on: `ShopDelta`, which did not exist
//! before — the command had no representation of "what this purchase changed",
//! only the post-mutation absolute row.

use gambling::{ShopDelta, ShopRow};
use serenity::all::UserId;

const fn row(coins: i64, gems: i64) -> ShopRow {
    let mut row = ShopRow::new(UserId::new(1));
    row.coins = coins;
    row.gems = gems;
    row
}

/// The DS-9 double-submit, at the value level.
///
/// User holds 10 000 coins. Two `/shop buy` invocations for a 4 000-coin item
/// are dispatched on separate tokio tasks and both read the pre-image.
///
/// - Old behaviour: both write the absolute `10 000 - 4 000` = 6 000. The user ends
///   with **two items and one charge** — 4 000 coins minted.
/// - Fixed behaviour: each write applies its own `-4 000` increment, so the second
///   lands on the first's result: 10 000 - 4 000 - 4 000 = 2 000.
#[test]
fn concurrent_purchase_is_not_lost() {
    let before = row(10_000, 0);

    let mut after = row(10_000, 0);
    after.coins -= 4_000;

    let delta = ShopDelta::between(&before, &after);
    assert_eq!(delta.coins, -4_000, "the delta must be the purchase's cost");

    // The sibling invocation committed its own -4 000 first.
    let current_in_db = 6_000;

    assert_eq!(current_in_db + delta.coins, 2_000, "both charges land");
    // The absolute write the finding describes, pinned so a regression to it fails.
    assert_eq!(after.coins, 6_000);
    assert_ne!(after.coins, 2_000, "absolute overwrite refunds one purchase");
}

/// A concurrent *credit* (a gift, a goal reward, a `/work` payout) landing
/// between the read and the write is likewise preserved by the increment and
/// erased by the absolute write.
#[test]
fn concurrent_credit_survives_the_purchase_write() {
    let before = row(10_000, 0);

    let mut after = row(10_000, 0);
    after.coins -= 4_000;

    let delta = ShopDelta::between(&before, &after);

    // +2 500 credited by another command in the window.
    assert_eq!(12_500 + delta.coins, 8_500, "the credit survives");
    assert_ne!(after.coins, 8_500, "absolute overwrite would drop the credit");
}

/// Goal rewards credited by `Dispatch::fire` (`add_coins(5_000)` /
/// `add_gems(1)`) are inside the delta — it is taken *after* the dispatch, so
/// they ride the same atomic increment rather than a separate write.
#[test]
fn goal_rewards_are_included_in_the_delta() {
    let before = row(10_000, 3);

    let mut after = row(10_000, 3);
    after.coins -= 4_000;
    // A goal completed while the purchase was dispatching.
    after.coins += 5_000;
    after.gems += 1;

    let delta = ShopDelta::between(&before, &after);

    assert_eq!(delta.coins, 1_000);
    assert_eq!(delta.gems, 1);
}

/// Mine purchases move a `gambling_mine` column, and the same lost-update
/// applies: two concurrent `miner` buys must both count.
#[test]
fn mine_column_purchase_is_an_increment() {
    let mut before = ShopRow::new(UserId::new(1));
    before.coins = 10_000;
    before.miners = 4;

    let mut after = ShopRow::new(UserId::new(1));
    after.coins = 10_000 - 100;
    after.miners = 4 + 3;

    let delta = ShopDelta::between(&before, &after);

    assert_eq!(delta.miners, 3);
    assert_eq!(delta.coins, -100);
    // Sibling invocation already committed +3, taking the row to 7.
    assert_eq!(7 + delta.miners, 10, "both purchases count");
    assert_ne!(after.miners, 10, "absolute overwrite would drop one");
}

/// The crafted currencies (`tech` / `utility` / `production`) are spendable in
/// the shop and live in the same row, so they are part of the delta too.
#[test]
fn crafted_currency_spend_is_in_the_delta() {
    let mut before = ShopRow::new(UserId::new(1));
    before.tech = 12;
    before.utility = 8;
    before.production = 5;

    let mut after = ShopRow::new(UserId::new(1));
    after.tech = 12 - 2;
    after.utility = 8 - 1;
    after.production = 5;

    let delta = ShopDelta::between(&before, &after);

    assert_eq!(delta.tech, -2);
    assert_eq!(delta.utility, -1);
    assert_eq!(delta.production, 0);
}

/// A purchase that changes nothing produces a no-op delta, so the write cannot
/// rewrite the row with stale values.
#[test]
fn unchanged_row_yields_a_zero_delta() {
    let before = row(10_000, 4);
    let delta = ShopDelta::between(&before, &row(10_000, 4));

    assert_eq!(delta, ShopDelta::default());
    assert!(delta.is_noop());
}

/// `is_mine_noop` gates the `gambling_mine` statement: a plain item purchase
/// must not touch (or lock) the mine row at all.
#[test]
fn item_only_purchase_leaves_the_mine_row_untouched() {
    let before = row(10_000, 0);

    let mut after = row(10_000, 0);
    after.coins -= 250;

    let delta = ShopDelta::between(&before, &after);

    assert!(delta.is_mine_noop(), "no mine column changed");
    assert!(!delta.is_noop(), "but coins did");
}

/// A mine purchase spending only coins still needs the mine statement.
#[test]
fn mine_purchase_is_not_a_mine_noop() {
    let mut before = ShopRow::new(UserId::new(1));
    before.coins = 10_000;

    let mut after = ShopRow::new(UserId::new(1));
    after.coins = 9_000;
    after.mines = 1;

    let delta = ShopDelta::between(&before, &after);

    assert!(!delta.is_mine_noop());
}
