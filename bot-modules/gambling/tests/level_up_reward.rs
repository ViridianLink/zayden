//! Tests for the chat-activity coin faucet and the bet ceiling it feeds.
//!
//! Chat XP was the largest faucet in the economy and the only one nobody
//! designed as one: every level-up silently paid `level * 1000` coins from
//! `bot/src/handler/message_create.rs`, outside both the `gambling` and
//! `levels` crates. Because the per-level payout is linear in level, the
//! *cumulative* payout is quadratic — a player reaching level 10 (~211
//! messages) collected 55,000 coins against a 1,000-coin starting balance,
//! while `/daily` pays 1,000 and `/work` pays 100-500.
//!
//! The reward now lives in `gambling` as [`level_up_reward`] so the curve is
//! reviewable and testable in-process. These tests pin the shape of that curve
//! and the `MaxBet` widening that stopped `level * 10_000` overflowing an i32.

use gambling::{MaxBet, Prestige, level_up_reward};

/// Cumulative coins from chat alone on the way to `level`.
fn cumulative_to(level: i64) -> i64 {
    (1..=level).map(level_up_reward).sum()
}

#[test]
fn reward_scales_with_level() {
    assert_eq!(level_up_reward(1), 100);
    assert_eq!(level_up_reward(10), 1_000);
    assert_eq!(level_up_reward(20), 2_000);
}

/// Levels are 1-indexed on the paying path (`accrue_message` returns the level
/// just reached), but the faucet must not pay out for a non-positive level if
/// that ever changes.
#[test]
fn non_positive_levels_pay_nothing() {
    assert_eq!(level_up_reward(0), 0);
    assert_eq!(level_up_reward(-1), 0);
}

/// The regression this file exists for: chat must stay an order of magnitude
/// below the deliberate faucets. Reaching level 10 takes roughly 211 messages;
/// under the old `level * 1000` curve that paid 55,000 coins, dwarfing the
/// 1,000-coin starting balance and ten days of `/daily`.
#[test]
fn chat_faucet_stays_below_the_deliberate_ones() {
    const START_AMOUNT: i64 = 1_000;

    let to_level_10 = cumulative_to(10);

    assert_eq!(to_level_10, 5_500);

    // The old curve, for contrast: an order of magnitude larger.
    assert_eq!(to_level_10 * 10, 55_000);

    // ~211 messages of chat is worth fewer than six `/daily` claims.
    assert!(
        to_level_10 < START_AMOUNT * 6,
        "chat out-earns /daily too quickly: {to_level_10} coins by level 10"
    );
}

/// The payout is quadratic in level, so it still compounds — this pins how fast,
/// to catch a coefficient bump that quietly re-inflates the late game.
#[test]
fn cumulative_reward_is_quadratic() {
    // 100 * L(L+1)/2
    for level in [1_i64, 10, 40, 100] {
        assert_eq!(cumulative_to(level), 50 * level * (level + 1));
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

#[test]
fn max_bet_has_a_floor_and_scales_with_level() {
    assert_eq!(Player { level: 0, prestige: 0 }.max_bet(), 10_000);
    assert_eq!(Player { level: 1, prestige: 0 }.max_bet(), 10_000);
    assert_eq!(Player { level: 10, prestige: 0 }.max_bet(), 100_000);
}

/// The prestige term compounds at 1.5x per level rather than adding 10%, so ten
/// prestiges are worth ~55x on the ceiling instead of 2x. That growth is what
/// pays for the upper mine rungs; see `tests/prestige_progression.rs` for the
/// curve itself.
#[test]
fn prestige_multiplies_the_bet_ceiling() {
    let base = Player { level: 10, prestige: 0 }.max_bet();
    let prestiged = Player { level: 10, prestige: 10 }.max_bet();

    assert_eq!(base, 100_000);
    assert_eq!(prestiged, 5_490_000);
    assert!(prestiged > base * 50, "ten prestiges should compound, not add");
}

/// `level * 10_000` was an `i32 * i32` product, wrapping (or panicking in debug)
/// above level 214,748. Widening to i64 first keeps the limit monotonic.
#[test]
fn max_bet_does_not_overflow_at_extreme_levels() {
    let ceiling = Player { level: i32::MAX, prestige: 0 }.max_bet();

    assert_eq!(ceiling, i64::from(i32::MAX) * 10_000);
    assert!(ceiling > Player { level: 214_748, prestige: 0 }.max_bet());
}
