//! Regression tests for gambling DS-14 — the wager-game (`GameRow`) site of the
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite class.
//!
//! DS-14 was that every wager game (`/roll`, `/coinflip`, `/rps`, `/tictactoe`,
//! blackjack, higher-or-lower) read the whole `GameRow`, deducted the bet and
//! added the payout **in memory**, then persisted the result **absolutely**
//! (`coins = EXCLUDED.coins, gems = EXCLUDED.gems`). `GameCache::check_and_set`
//! gates only the same user's repeat *game* plays, so any other command
//! crediting `gambling.coins` in the read→write window — `/daily`, `/work`,
//! `/dig`, `/shop sell`, an inbound `/send` — was erased by that write.
//!
//! The end-to-end command path needs a live `PgPool` plus a Discord interaction,
//! for which this crate has no harness (audit CC-6), and the balance floor
//! itself lives in SQL. What is testable in-process is what the fix introduces:
//! `GameDelta` — the *change* a game made, applied atomically at write time
//! (`col = col + delta`) rather than as an absolute post-image.

use gambling::{Coins, GameDelta, GameRow, Gems};
use serenity::all::UserId;

const USER: UserId = UserId::new(1);

const fn row_with(coins: i64, gems: i64) -> GameRow {
    let mut row = GameRow::new(USER);
    row.coins = coins;
    row.gems = gems;
    row
}

/// A won `/coinflip` — 500 staked, 1,000 paid out — is a `+500` increment
/// against whatever the row holds, not a post-image of the stale read.
#[test]
fn game_delta_is_signed_increments_not_a_post_image() {
    let before = row_with(10_000, 3);

    let mut after = before.clone();
    after.bet(500);
    after.add_coins(1_000);

    let delta = GameDelta::between(&before, &after);

    assert_eq!(delta.coins, 500);
    // An untouched column contributes nothing to the write.
    assert_eq!(delta.gems, 0);
}

/// The finding's failure scenario: a `/daily` credit lands between the game's
/// read and its write.
#[test]
fn concurrent_credit_survives_the_game_write() {
    let before = row_with(10_000, 0);

    // The player loses a 500 bet.
    let mut after = before.clone();
    after.bet(500);

    let delta = GameDelta::between(&before, &after);
    assert_eq!(delta.coins, -500);

    // Meanwhile `/daily` credited 5,000 atomically, after the game's read.
    let live_balance = 15_000;

    assert_eq!(live_balance + delta.coins, 14_500);
    assert_ne!(after.coins, 14_500, "absolute write would clobber the /daily");
    assert_eq!(after.coins, 9_500, "…losing exactly the 5,000 credit");
}

/// Two games settling in the same tick from the same pre-image. Both bets are
/// charged once the writes are increments; the absolute write charged one.
#[test]
fn two_games_settling_together_are_both_charged() {
    let before = row_with(10_000, 0);

    let mut first = before.clone();
    first.bet(500);
    let mut second = before.clone();
    second.bet(300);

    let first_delta = GameDelta::between(&before, &first);
    let second_delta = GameDelta::between(&before, &second);

    assert_eq!(before.coins + first_delta.coins + second_delta.coins, 9_200);

    // The absolute write the finding describes: the later writer persisted its
    // own `pre-image − its own bet`, so the other game was played for free.
    assert_eq!(second.coins, 9_700);
}

/// Goal rewards are granted by `Dispatch` **into the in-memory row**
/// (`row.add_coins(5_000)` / `row.add_gems(1)`) and rely on the game's write to
/// persist them, so they have to travel in the delta — on both columns.
#[test]
fn dispatch_goal_reward_travels_in_the_delta() {
    let before = row_with(10_000, 2);

    let mut after = before.clone();
    after.bet(500);
    // Daily goal completed, then the last goal of the day.
    after.add_coins(5_000);
    after.add_gems(1);
    // …and the game itself paid out.
    after.add_coins(1_000);

    let delta = GameDelta::between(&before, &after);

    assert_eq!(delta.coins, 5_500);
    assert_eq!(delta.gems, 1);
}

/// The tic-tac-toe stake: each player's accepted wager is a plain coin debit,
/// and both are committed in one transaction.
#[test]
fn stake_delta_is_the_negated_bet() {
    let delta = GameDelta::coins(-250);

    assert_eq!(delta.coins, -250);
    assert_eq!(delta.gems, 0);
    assert_eq!(delta, GameDelta { coins: -250, gems: 0 });
}

/// A game that neither wins nor costs anything (a `/tictactoe` challenge that is
/// only offered, a blackjack draw) writes nothing — the guard passes it through
/// so the caller still gets the authoritative balance back.
#[test]
fn a_settled_draw_is_a_zero_delta() {
    let before = row_with(10_000, 0);

    let mut after = before.clone();
    after.bet(500);
    after.add_coins(500);

    assert_eq!(GameDelta::between(&before, &after), GameDelta::default());
}
