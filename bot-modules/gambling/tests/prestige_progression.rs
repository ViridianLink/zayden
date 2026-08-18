//! Tests for the prestige progression curve — the requirement to prestige, and
//! the multipliers prestiging buys.
//!
//! Two defects motivated these. First, `req_miners()` ended in
//! `required_miners *= self.universes`, and `do_prestige()` zeroes `universes`,
//! so from prestige 15 the requirement collapsed to `10^8 * 0 == 0` and
//! prestige became free and infinitely repeatable. The requirement must
//! therefore be derived from `prestige` alone, never from a field the reset
//! wipes.
//!
//! Second, the mine ladder's upper rungs were decorative. Each rung caps the
//! rung below it at `10 * (below + 1)`, so there is a hard miner ceiling for
//! every rung you do not own. The old tiers (10^7 at P>=5, 10^8 at P>=10) both
//! landed just *under* their ceilings — 11,111,110 without a galaxy and
//! 111,111,110 without a universe — so no prestige level ever forced either
//! purchase. The tiers now clear those ceilings, which is what makes the
//! Universe the actual capstone rather than an ornament.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord
//! interaction, for which this crate has no harness (audit CC-6), so these
//! exercise the pure functions the command calls.

use gambling::{
    MAX_SCALING_PRESTIGE,
    MaxBet,
    Prestige,
    PrestigeRow,
    miner_cap_without,
};

fn row_at_prestige(prestige: i64) -> PrestigeRow {
    PrestigeRow { prestige, ..Default::default() }
}

/// Number of ladder rungs between the miners and the rung being denied.
const WITHOUT_PLANET: u32 = 4;
const WITHOUT_SOLAR_SYSTEM: u32 = 5;
const WITHOUT_GALAXY: u32 = 6;
const WITHOUT_UNIVERSE: u32 = 7;

#[test]
fn miner_ceilings_follow_the_ladder_recurrence() {
    assert_eq!(miner_cap_without(WITHOUT_PLANET), 111_110);
    assert_eq!(miner_cap_without(WITHOUT_SOLAR_SYSTEM), 1_111_110);
    assert_eq!(miner_cap_without(WITHOUT_GALAXY), 11_111_110);
    assert_eq!(miner_cap_without(WITHOUT_UNIVERSE), 111_111_110);
}

/// The regression that matters: a zero requirement means free, unbounded
/// prestige, and with it unbounded gems.
#[test]
fn requirement_is_never_zero_even_once_universes_are_wiped() {
    for prestige in [0, 4, 5, 9, 10, 14, 15, 16, 50, 500] {
        let row = row_at_prestige(prestige);

        assert_eq!(
            row.universes, 0,
            "a freshly reset row holds no universes — this is the state the old \
             `*= self.universes` multiplied by"
        );
        assert!(
            row.req_miners() > 0,
            "prestige {prestige} must still cost something"
        );
    }
}

#[test]
fn requirement_never_decreases_with_prestige() {
    let mut previous = 0;

    for prestige in 0..40 {
        let required = row_at_prestige(prestige).req_miners();

        assert!(
            required >= previous,
            "requirement fell from {previous} to {required} at prestige {prestige}"
        );

        previous = required;
    }
}

/// Each tier has to clear the ceiling of the rung it is meant to force,
/// otherwise that rung stays optional and the ladder never advances.
#[test]
fn each_tier_forces_the_next_mine_rung() {
    let forced = [
        (0, WITHOUT_PLANET, "a planet"),
        (5, WITHOUT_SOLAR_SYSTEM, "a solar system"),
        (10, WITHOUT_GALAXY, "a galaxy"),
        (15, WITHOUT_UNIVERSE, "a universe"),
    ];

    for (prestige, rungs_above, rung) in forced {
        let required = row_at_prestige(prestige).req_miners();
        let ceiling = miner_cap_without(rungs_above);

        assert!(
            required > ceiling,
            "prestige {prestige} needs {required} miners but {ceiling} are \
             reachable without {rung} — the purchase stays optional"
        );
    }
}

/// The flip side: a tier must not force a rung beyond the one it targets, or
/// the early game jumps straight to the top of the ladder.
#[test]
fn early_tiers_do_not_force_the_whole_ladder() {
    assert!(
        row_at_prestige(0).req_miners() <= miner_cap_without(WITHOUT_SOLAR_SYSTEM),
        "the first prestige should stop at planets"
    );
    assert!(
        row_at_prestige(5).req_miners() <= miner_cap_without(WITHOUT_GALAXY),
        "the second tier should stop at solar systems"
    );
    assert!(
        row_at_prestige(10).req_miners() <= miner_cap_without(WITHOUT_UNIVERSE),
        "the third tier should stop at galaxies"
    );
}

/// Prestige 0 must be a no-op multiplier, or every existing balance shifts.
#[test]
fn prestige_zero_leaves_the_multipliers_neutral() {
    let row = row_at_prestige(0);

    assert_eq!(row.prestige_mult_100(), 100);
    assert_eq!(row.prestige_mult_10(), 10);
}

/// The multipliers were linear (`100 + P`, `10 + P`), which over fifteen full
/// resets bought +15% mine income — far too little to bridge the late ladder.
/// Geometric growth is what makes each cycle meaningfully shorter than the last.
#[test]
fn multipliers_compound_rather_than_adding() {
    let ten = row_at_prestige(10);

    assert!(
        ten.prestige_mult_100() > 100 + 10,
        "mine income must beat the old linear curve"
    );
    assert!(
        ten.prestige_mult_10() > 10 + 10,
        "dig and max bet must beat the old linear curve"
    );

    // 1.25^10 ~= 9.31 and 1.5^10 ~= 57.7, with truncation each step pulling
    // slightly under.
    assert!((880..=931).contains(&ten.prestige_mult_100()));
    assert!((540..=577).contains(&ten.prestige_mult_10()));
}

#[test]
fn multipliers_are_monotonic_up_to_the_ceiling() {
    let mut previous = (0, 0);

    for prestige in 0..=MAX_SCALING_PRESTIGE {
        let row = row_at_prestige(prestige);
        let current = (row.prestige_mult_100(), row.prestige_mult_10());

        assert!(
            current.0 >= previous.0 && current.1 >= previous.1,
            "multipliers fell at prestige {prestige}"
        );

        previous = current;
    }
}

/// Geometric curves leave `i64` fast, so they are held flat past the ceiling.
/// Without this an extreme prestige level would wrap a payout negative.
#[test]
fn multipliers_stop_growing_past_the_ceiling() {
    let capped = row_at_prestige(MAX_SCALING_PRESTIGE);

    for beyond in [MAX_SCALING_PRESTIGE + 1, MAX_SCALING_PRESTIGE * 10, i64::MAX] {
        let row = row_at_prestige(beyond);

        assert_eq!(row.prestige_mult_100(), capped.prestige_mult_100());
        assert_eq!(row.prestige_mult_10(), capped.prestige_mult_10());
    }
}

struct Player {
    level: i32,
    prestige: i64,
}

impl Prestige for Player {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl MaxBet for Player {
    fn level(&self) -> i32 {
        self.level
    }
}

/// The bet ceiling is the income lever, so it has to survive both an extreme
/// level and an extreme prestige without wrapping.
#[test]
fn max_bet_does_not_overflow_at_either_extreme() {
    for prestige in [0, 1, MAX_SCALING_PRESTIGE, i64::MAX] {
        let player = Player { level: i32::MAX, prestige };

        assert!(player.max_bet() > 0, "max bet wrapped at prestige {prestige}");
    }
}

#[test]
fn max_bet_still_floors_and_scales_with_level() {
    assert_eq!(Player { level: 0, prestige: 0 }.max_bet(), 10_000);
    assert_eq!(Player { level: 1, prestige: 0 }.max_bet(), 10_000);
    assert_eq!(Player { level: 10, prestige: 0 }.max_bet(), 100_000);
}
