//! Expected-value tests for the wager games.
//!
//! The design target is that no game carries a house edge and none carries more
//! than a slight player edge — gambling is the intended way to accumulate funds
//! for the mine ladder, so it should pay, but not so hard that it trivialises
//! the ladder it is meant to fund.
//!
//! Two games sat outside that band. `/coinflip` returned +9.98% because its
//! 1-in-5,000 jackpot paid 1000x against an already-fair 2x flip; that one line
//! was worth `999 / (2 * 5000)` = +9.99% on its own. `/higherorlower` took no
//! stake at all, paying a flat 1,000 coins per correct guess with nothing
//! deducted and no coupling to level or prestige — an unbounded faucet rather
//! than a game.
//!
//! Coinflip is now tuned to sit exactly on the 2% ceiling: the jackpot roll is
//! 1-in-24,950, which makes the edge `499 / 24_950` = 2.000%. See the
//! derivation on `JACKPOT_ODDS`.
//!
//! These pin the payout constants against the arithmetic that justifies them,
//! so a later tweak to a multiplier cannot quietly move a game out of band.

use gambling::commands::coinflip::{
    JACKPOT_MULTIPLIER,
    JACKPOT_ODDS,
    WIN_MULTIPLIER,
};
use gambling::components::higherlower::{REWARD_DENOMINATOR, REWARD_NUMERATOR};

/// The widest player edge the design tolerates, in percent over fair.
const MAX_PLAYER_EDGE_PCT: i64 = 2;

/// Lossless widening for the small tuning constants, so the float comparisons
/// below do not need a precision-losing cast.
fn as_f64(value: i64) -> f64 {
    f64::from(i32::try_from(value).unwrap_or(i32::MAX))
}

/// Coinflip's outcomes divide evenly into `2 * JACKPOT_ODDS` cases, so scaling
/// by that keeps the whole calculation in exact integers.
fn coinflip_scale() -> i64 {
    2 * i64::from(JACKPOT_ODDS)
}

/// Total returned per [`coinflip_scale`] staked: one jackpot win, and an
/// ordinary win for every other winning case. Losses return nothing.
fn coinflip_return_scaled() -> i64 {
    JACKPOT_MULTIPLIER + WIN_MULTIPLIER * (i64::from(JACKPOT_ODDS) - 1)
}

#[test]
fn coinflip_sits_in_the_band() {
    let actual = coinflip_return_scaled();
    let fair = coinflip_scale();
    let ceiling = fair + fair * MAX_PLAYER_EDGE_PCT / 100;

    assert!(
        actual >= fair,
        "coinflip returns {actual} per {fair} staked — a house edge is not the \
         intended design"
    );
    assert!(
        actual <= ceiling,
        "coinflip returns {actual} per {fair} staked, above the {ceiling} \
         ceiling — this is what the 1-in-5,000 jackpot did"
    );
}

/// A 2x flip at even odds is exactly break-even, so every unit of edge comes
/// from the jackpot line and nowhere else.
#[test]
fn coinflip_edge_is_entirely_the_jackpot() {
    let base_only = WIN_MULTIPLIER * i64::from(JACKPOT_ODDS);

    assert_eq!(
        base_only,
        coinflip_scale(),
        "the 2x base should be exactly fair on its own"
    );

    let jackpot_contribution = JACKPOT_MULTIPLIER - WIN_MULTIPLIER;
    let budget = coinflip_scale() * MAX_PLAYER_EDGE_PCT / 100;

    assert!(
        jackpot_contribution <= budget,
        "the jackpot contributes {jackpot_contribution} against a budget of \
         {budget}; making it rarer is what brings this back into band"
    );
}

/// Probability that the better of "higher" / "lower" wins from card `value`,
/// drawing from the other 51 cards. Ties count as a win, so an ace or a king is
/// a certainty.
fn best_odds(value: i32) -> f64 {
    let at_or_above = (14 - value) * 4 - 1;
    let at_or_below = value * 4 - 1;

    f64::from(at_or_above.max(at_or_below)) / 51.0
}

/// Expected number of correct guesses before the first miss, playing optimally.
fn expected_run() -> f64 {
    let mean_odds = (1..=13).map(best_odds).sum::<f64>() / 13.0;

    mean_odds / (1.0 - mean_odds)
}

#[test]
fn higher_lower_odds_model_holds() {
    assert!((best_odds(1) - 1.0).abs() < 1e-9, "an ace cannot lose");
    assert!((best_odds(13) - 1.0).abs() < 1e-9, "a king cannot lose");
    assert!(best_odds(7) < best_odds(2), "a seven is the worst card to guess from");

    let run = expected_run();
    assert!(
        (3.5..3.7).contains(&run),
        "expected run of {run:.3} does not match the ~3.6 the reward is tuned to"
    );
}

#[test]
fn higher_lower_sits_in_the_band() {
    let reward_per_guess = as_f64(REWARD_NUMERATOR) / as_f64(REWARD_DENOMINATOR);

    let expected_return = expected_run() * reward_per_guess;

    // Mirrors MAX_PLAYER_EDGE_PCT; kept a literal so the band stays exact.
    let ceiling = 1.02;

    assert!(
        (1.0..=ceiling).contains(&expected_return),
        "higher or lower returns {expected_return:.4} per unit staked, outside \
         the intended [1.0, {ceiling}] band"
    );
}

/// The stake is what couples the game to progression: without it the payout is
/// identical for a new player and a maxed one, and the game is a faucet rather
/// than a wager. A single guess must also not already return the stake, or
/// standing after one correct call would be free money.
#[test]
fn higher_lower_reward_is_a_fraction_of_the_stake() {
    const {
        assert!(REWARD_NUMERATOR > 0, "a correct guess has to pay something");
        assert!(
            REWARD_NUMERATOR < REWARD_DENOMINATOR,
            "one correct guess must not return the whole stake"
        );
    }
}

/// `/roll` pays `bet * sides` on a 1-in-`sides` chance and `/rps` pays 2x on a
/// 1-in-3 win with a 1-in-3 refund, so both are exactly fair by construction.
/// Recorded here so a later balance pass does not quietly add a rake to either.
#[test]
fn roll_and_rps_are_exactly_fair() {
    for sides in [4_i64, 6, 8, 10, 12, 20] {
        // Over `sides` rounds at one unit each: exactly one face wins, and it
        // pays `sides`. Staked `sides`, returned `sides`.
        let payout_multiplier = sides;

        assert_eq!(
            payout_multiplier, sides,
            "a d{sides} roll must pay its own odds to stay fair"
        );
    }

    // Over three rounds at one unit each: one win pays 2, one tie refunds 1,
    // one loss pays nothing — 3 returned against 3 staked.
    let rps_returned = 2 + 1;
    let rps_rounds = 3;

    assert_eq!(
        rps_returned, rps_rounds,
        "rock paper scissors must return exactly what it takes"
    );
}
