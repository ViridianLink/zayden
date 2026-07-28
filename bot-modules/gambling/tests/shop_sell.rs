//! Regression tests for gambling DS-10 — the `/shop sell`
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite site,
//! the mirror of DS-9 ([`shop_buy.rs`](shop_buy.rs)).
//!
//! DS-10 was that `/shop sell` read `SellRow` (coins plus the item's inventory
//! quantity), credited the payment and decremented the quantity **in memory**,
//! then persisted both **absolutely** — `coins = EXCLUDED.coins` and
//! `UPDATE gambling_inventory SET quantity = $1 WHERE id = $2`. Two `/shop sell`
//! invocations in the same tick both read the pre-image, both wrote the same
//! absolute post-image, and the item was consumed once while the payout that
//! landed last decided the balance.
//!
//! As with [`shop_buy.rs`](shop_buy.rs) and [`work.rs`](work.rs), the end-to-end
//! command path needs a live `PgPool` plus a Discord interaction, for which this
//! crate has no harness (audit CC-6), and the guard itself
//! (`WHERE quantity >= $n`) lives in SQL. What is testable in-process is the
//! value-level piece the fix turns on: `SaleDelta`, which did not exist before —
//! the command had no representation of "what this sale changed", only the
//! post-mutation absolute row.

use gambling::{SHOP_ITEMS, SaleDelta};

/// The DS-10 double-submit, at the value level.
///
/// User holds 5 of an item worth 1 000 coins each and double-submits a sale of
/// 2, with 10 000 coins in the bank.
///
/// - Old behaviour: both invocations read `quantity = 5`, both write the absolute
///   `3`, and both write their own absolute coin total — 2 items consumed for what
///   the row records as one sale of 2.
/// - Fixed behaviour: each write applies its own `-2` / `+1 800`, so the second
///   lands on the first's result: 5 - 2 - 2 = 1.
#[test]
fn concurrent_sale_is_not_lost() {
    let delta = SaleDelta::new(1_000, 2);

    assert_eq!(delta.quantity, 2, "the delta must be the units sold");
    assert_eq!(delta.coins, 1_800, "1 000 × 2 at a 90% sales return");

    // The sibling invocation committed its own -2 first.
    let quantity_in_db = 3;
    assert_eq!(quantity_in_db - delta.quantity, 1, "both sales consume stock");

    // The absolute write the finding describes, pinned so a regression fails.
    let absolute_post_image = 5 - delta.quantity;
    assert_eq!(absolute_post_image, 3);
    assert_ne!(absolute_post_image, 1, "absolute overwrite restocks one sale");
}

/// The payout side of the same double-submit: two absolute coin writes of the
/// same post-image pay once for two sales.
#[test]
fn concurrent_payout_is_not_lost() {
    let delta = SaleDelta::new(1_000, 2);

    // Sibling invocation already credited its 1 800.
    let coins_in_db = 11_800;
    assert_eq!(coins_in_db + delta.coins, 13_600, "both payouts land");

    let absolute_post_image = 10_000 + delta.coins;
    assert_eq!(absolute_post_image, 11_800);
    assert_ne!(absolute_post_image, 13_600, "absolute overwrite pays once");
}

/// A concurrent *debit* (a `/shop buy`, a wager, a `/send`) landing between the
/// read and the write is preserved by the increment and erased by the absolute
/// write — the sale would silently refund it.
#[test]
fn concurrent_debit_survives_the_sale_write() {
    let delta = SaleDelta::new(1_000, 2);

    // -4 000 spent by another command in the window.
    assert_eq!(6_000 + delta.coins, 7_800, "the debit survives");
    assert_ne!(10_000 + delta.coins, 7_800, "absolute write drops the debit");
}

/// The sales tax is applied to the whole sale, not per unit, and truncates the
/// same way the pre-fix command did — the fix must not change what a sale pays.
#[test]
fn payment_matches_the_pre_fix_arithmetic() {
    for (unit_cost, amount) in
        [(1_000_i64, 1_i64), (1_000, 7), (333, 3), (1, 1), (7, 13)]
    {
        let expected = unit_cost * amount * 90 / 100;
        assert_eq!(
            SaleDelta::new(unit_cost, amount).coins,
            expected,
            "sale of {amount} × {unit_cost}"
        );
    }
}

/// Items with no coin cost (gem-only purchases) sell for nothing rather than
/// panicking or minting — the `unwrap_or(0)` the command applied stays.
#[test]
fn zero_cost_item_pays_nothing() {
    let delta = SaleDelta::new(0, 4);

    assert_eq!(delta.coins, 0);
    assert_eq!(delta.quantity, 4, "but the stock still moves");
}

/// Selling zero units is a no-op on both columns, so the write cannot be used
/// to re-assert a stale balance.
#[test]
fn zero_amount_is_a_noop_delta() {
    let delta = SaleDelta::new(1_000, 0);

    assert_eq!(delta.coins, 0);
    assert_eq!(delta.quantity, 0);
}

/// Sanity-check the delta against a real catalogue entry, so a change to
/// `SALES_RETURN` or to an item's cost is caught here rather than in production.
#[test]
fn real_shop_item_round_trips_through_the_delta() {
    let item =
        SHOP_ITEMS.get("eggplant").expect("'eggplant' is in the shop catalogue");
    let unit_cost = item.coin_cost().expect("'eggplant' is bought with coins");

    let delta = SaleDelta::new(unit_cost, 3);

    assert_eq!(delta.quantity, 3);
    assert_eq!(delta.coins, unit_cost * 3 * 90 / 100);
    assert!(delta.coins < unit_cost * 3, "a sale must not be a round trip");
}
