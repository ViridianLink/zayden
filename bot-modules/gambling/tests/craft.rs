//! Regression tests for gambling DS-13 — the `/craft` site of the
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite class.
//!
//! DS-13 was that `/craft` read the whole `CraftRow` (all ten `gambling_mine`
//! currency columns), deducted the recipe's raw-material costs and added the
//! crafted pack **in memory**, then persisted the result **absolutely**
//! (`coal = EXCLUDED.coal, … production = EXCLUDED.production`). Two crafts in
//! the same tick therefore read the same pre-image and the later write erased
//! the earlier one — one craft's materials were never charged — and a concurrent
//! `/dig`/`/work`/`/shop buy` write to any of those columns was clobbered.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord interaction,
//! for which this crate has no harness (audit CC-6), and the per-column floor
//! guard itself lives in SQL. What is testable in-process is what the fix
//! introduces: `CraftDelta` — the *change* a craft made, applied atomically at
//! write time (`col = col + delta`) rather than as an absolute post-image.

use gambling::{CraftDelta, CraftRow};
use serenity::all::UserId;

const USER: UserId = UserId::new(1);

/// A Tech pack costs 10 coal + 5 iron and yields 1 tech: the delta is those
/// three changes, each a signed increment against whatever the row holds.
#[test]
fn craft_delta_is_signed_increments_not_a_post_image() {
    let mut before = CraftRow::new(USER);
    before.coal = 100;
    before.iron = 40;

    // In-memory mutation the command performs for one Tech pack.
    let mut after = before.clone();
    after.coal -= 10;
    after.iron -= 5;
    after.tech += 1;

    let delta = CraftDelta::between(&before, &after);

    assert_eq!(delta.coal, -10);
    assert_eq!(delta.iron, -5);
    assert_eq!(delta.tech, 1);
    // Untouched columns contribute nothing.
    assert_eq!(delta.gold, 0);
    assert_eq!(delta.utility, 0);
    assert_eq!(delta.production, 0);
}

/// Two Tech crafts in the same tick, both reading the same pre-image.
///
/// Applying both deltas to the live row charges both (the fixed behaviour);
/// the absolute write charged only once, since each craft wrote `pre-image −
/// one cost`, so the later writer's post-image was all that survived.
#[test]
fn concurrent_crafts_are_each_charged() {
    let mut before = CraftRow::new(USER);
    before.coal = 100;
    before.iron = 40;

    // Both invocations mutate their own copy of the same read.
    let mut first = before.clone();
    first.coal -= 10;
    first.iron -= 5;
    first.tech += 1;
    let mut second = before.clone();
    second.coal -= 10;
    second.iron -= 5;
    second.tech += 1;

    let first_delta = CraftDelta::between(&before, &first);
    let second_delta = CraftDelta::between(&before, &second);

    // Fixed: both deltas land on the shared row → both packs charged.
    assert_eq!(before.coal + first_delta.coal + second_delta.coal, 80);
    assert_eq!(before.iron + first_delta.iron + second_delta.iron, 30);
    assert_eq!(before.tech + first_delta.tech + second_delta.tech, 2);

    // The absolute write the finding describes: the second craft persisted its
    // own `pre-image − one cost`, so only one Tech pack's cost was ever charged.
    assert_ne!(second.coal, 80, "absolute write charges only one craft's coal");
    assert_eq!(second.coal, 90);
    assert_eq!(second.tech, 1, "…and only one pack is delivered");
}

/// A concurrent ore gain that lands between the craft's read and its write
/// survives, because the craft applies increments rather than an absolute row.
#[test]
fn concurrent_ore_gain_survives_the_craft_write() {
    let mut before = CraftRow::new(USER);
    before.coal = 100;

    // The craft consumes 10 coal in memory.
    let mut after = before.clone();
    after.coal -= 10;

    let delta = CraftDelta::between(&before, &after);
    assert_eq!(delta.coal, -10);

    // A concurrent `/dig` credited +25 coal after the craft's read.
    let current_in_db = 125;

    assert_eq!(current_in_db + delta.coal, 115);
    assert_ne!(after.coal, 115, "absolute overwrite would clobber the +25 dig");
    assert_eq!(after.coal, 90, "…losing exactly the concurrent ore gain");
}

/// Every currency column travels in the delta, so a multi-ingredient recipe
/// (Utility: 15 coal + 10 gold + 5 diamonds + 1 emerald → 1 utility) cannot lose
/// an ingredient charge or the produced pack to a concurrent write.
#[test]
fn every_currency_column_is_carried() {
    let mut before = CraftRow::new(USER);
    before.coal = 50;
    before.gold = 50;
    before.diamonds = 20;
    before.emeralds = 5;

    let mut after = before.clone();
    after.coal -= 15;
    after.gold -= 10;
    after.diamonds -= 5;
    after.emeralds -= 1;
    after.utility += 1;

    let delta = CraftDelta::between(&before, &after);

    assert_eq!(
        (delta.coal, delta.gold, delta.diamonds, delta.emeralds, delta.utility),
        (-15, -10, -5, -1, 1)
    );
    // The un-spent ore columns stay out of the write entirely.
    assert_eq!(delta.iron, 0);
    assert_eq!(delta.redstone, 0);
    assert_eq!(delta.lapis, 0);
}

/// Crafting a multiple of a pack scales every ingredient and the yield linearly.
#[test]
fn crafting_multiple_packs_scales_the_delta() {
    let mut before = CraftRow::new(USER);
    before.coal = 100;
    before.iron = 100;

    // 3 Tech packs: 3×(10 coal + 5 iron) → 3 tech.
    let mut after = before.clone();
    after.coal -= 30;
    after.iron -= 15;
    after.tech += 3;

    let delta = CraftDelta::between(&before, &after);

    assert_eq!(delta.coal, -30);
    assert_eq!(delta.iron, -15);
    assert_eq!(delta.tech, 3);
}
