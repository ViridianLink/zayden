//! Regression tests for gambling DS-15 — the `/prestige` site of the
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite
//! class, and the last one in the module.
//!
//! DS-15 was that `confirm_prestige` read the whole `PrestigeRow`, and
//! `do_prestige()` folded the gem award into the snapshot
//! (`self.gems += self.prestige`), which `PrestigeManager::save` then persisted
//! **absolutely** (`gems = EXCLUDED.gems`). A concurrent `/work`, `/dig`,
//! `/shop sell` or wager-game payout — all of which credit gems atomically —
//! landing between the read and the write was therefore erased.
//!
//! `coins` and `stamina` are *not* part of the defect: prestige deliberately
//! resets them (back to the starting balance, and to a full stamina bar so the
//! player can keep playing straight after prestiging), so an absolute write is
//! the correct semantics for those two.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord interaction,
//! for which this crate has no harness (audit CC-6), and the increment itself
//! lives in SQL. What is testable in-process is what the fix introduces:
//! `do_prestige` returns the gem **award** — the amount to add to whatever the
//! live row holds — rather than a post-image of the `gems` column.

use gambling::PrestigeRow;
use serenity::all::UserId;
use zayden_core::as_i64;

const USER: UserId = UserId::new(1);

fn row_at_prestige(prestige: i64, gems: i64) -> PrestigeRow {
    PrestigeRow { user_id: as_i64(USER.get()), gems, prestige, ..Default::default() }
}

/// The award is the *new* prestige level, returned rather than folded into the
/// snapshot's `gems`, so the write can be `gems = gems + award`.
#[test]
fn do_prestige_returns_the_award_and_leaves_the_snapshot_gems_alone() {
    let mut row = row_at_prestige(0, 40);

    let award = row.do_prestige();

    assert_eq!(row.prestige, 1, "prestige level advances");
    assert_eq!(award, 1, "…and the award is the new level");
    assert_eq!(
        row.gems, 40,
        "the snapshot's gems must stay untouched — folding the award in here is \
         what made the persisted value an absolute post-image"
    );
}

/// The core DS-15 scenario: a gem credit that commits between the prestige read
/// and the prestige write survives, because the award is an increment applied to
/// the live row rather than a post-image computed from the stale snapshot.
#[test]
fn concurrent_gem_credit_survives_the_prestige_write() {
    let mut snapshot = row_at_prestige(0, 40);

    let award = snapshot.do_prestige();

    // A concurrent `/work` credited +5 gems after the prestige read.
    let live_gems = 45;

    assert_eq!(live_gems + award, 46, "fixed: the +5 and the award both land");
    assert_ne!(
        snapshot.gems + award,
        46,
        "the absolute write would clobber the concurrent /work credit"
    );
    assert_eq!(snapshot.gems + award, 41, "…losing exactly the 5 gems");
}

/// The award scales with the prestige level, so a high-prestige player's larger
/// award is still an increment — the bug's blast radius grew with the level.
#[test]
fn the_award_scales_with_the_prestige_level() {
    for before in [0, 1, 4, 9, 15, 99] {
        let mut row = row_at_prestige(before, 1_000);

        let award = row.do_prestige();

        assert_eq!(award, before + 1);
        assert_eq!(row.gems, 1_000, "snapshot gems never move");
    }
}

/// `coins` is a deliberate absolute reset, not a lost update: prestige puts the
/// player back to the same starting balance whatever they held before, so the
/// post-image does not depend on the pre-image.
#[test]
fn coins_are_reset_absolutely_not_incremented() {
    let mut rich = row_at_prestige(3, 0);
    rich.coins = 9_999_999;
    let mut broke = row_at_prestige(3, 0);
    broke.coins = 0;

    let _ = rich.do_prestige();
    let _ = broke.do_prestige();

    assert_eq!(rich.coins, broke.coins, "the reset ignores the pre-image");
    assert!(rich.coins > 0, "…and it is the starting balance, not zero");
}

/// Prestige wipes the whole mine so progress restarts from zero — the boosts,
/// not the resources, are what carry over.
#[test]
fn the_mine_is_wiped_by_the_reset() {
    let mut row = row_at_prestige(2, 0);
    row.miners = 5_000;
    row.mines = 400;
    row.land = 30;
    row.coal = 1_234;
    row.diamonds = 7;
    row.production = 42;

    let _ = row.do_prestige();

    assert_eq!(
        (row.miners, row.mines, row.land, row.coal, row.diamonds, row.production),
        (0, 0, 0, 0, 0, 0)
    );
}
