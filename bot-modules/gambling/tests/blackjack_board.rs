//! Round-trip tests for the blackjack board, which is the game's state
//! transport.
//!
//! Blackjack has no server-side session: between one button press and the next,
//! the only record of the round is the message itself. The board used to encode
//! that as prose — headings located by name, and a `"▶ "` marker character glued
//! onto the heading of whichever hand was live. Any edit to the wording broke a
//! round already in flight, silently, because the parse simply failed to find
//! what it was looking for.
//!
//! It is now structural: one [section] per hand, and the accessory badge's
//! custom id says which hand is active. Nothing in the parse reads prose. These
//! tests drive the real wire path — serialise the builders exactly as serenity
//! sends them, deserialise into the model types exactly as Discord returns
//! them, and parse that back — so a change to either half shows up here.
//!
//! [section]: https://docs.discord.com/developers/components/reference#section

use gambling::Result;
use gambling::games::blackjack::{GameDetails, in_play_board};
use serenity::all::{ContainerComponent, EmojiId};
use zayden_core::EmojiCache;

/// The names `card_deck` builds its 52 ids from, plus the two the board renders
/// directly.
fn emojis() -> EmojiCache {
    const SUITS: [&str; 4] = ["clubs", "diamonds", "hearts", "spades"];
    const VALUES: [&str; 13] =
        ["A", "02", "03", "04", "05", "06", "07", "08", "09", "10", "J", "Q", "K"];

    // Ids are arbitrary but must be distinct and non-zero — the deck is keyed
    // on them.
    SUITS
        .iter()
        .flat_map(|suit| VALUES.iter().map(move |value| format!("{suit}_{value}")))
        .chain(["heads".to_string(), "card_back".to_string()])
        .zip(1u64..)
        .map(|(name, id)| (name, EmojiId::new(id)))
        .collect()
}

/// Sends a board the way serenity does and reads it back the way Discord
/// returns it.
fn round_trip(emojis: &EmojiCache, game: &GameDetails) -> Result<GameDetails> {
    let board = in_play_board(emojis, game)?;

    let json = serde_json::to_string(&board).unwrap_or_default();

    let parsed =
        serde_json::from_str::<Vec<ContainerComponent>>(&json).unwrap_or_default();

    GameDetails::from_components(emojis, &parsed)
}

/// Ids assigned by [`emojis`]: clubs runs first, so these are the ace, the two
/// and the three of clubs.
const ACE: u64 = 1;
const TWO: u64 = 2;
const THREE: u64 = 3;
const KING: u64 = 13;

const fn card(id: u64) -> EmojiId {
    EmojiId::new(id)
}

fn opening_hand(bet: i64) -> GameDetails {
    GameDetails::new(bet, vec![card(TWO), card(THREE)], card(KING))
}

#[test]
fn a_single_hand_survives_the_round_trip() {
    let emojis = emojis();
    let game = opening_hand(2_500);

    let parsed = round_trip(&emojis, &game).unwrap();

    assert_eq!(parsed.bet(), 2_500);
    assert_eq!(parsed.hands(), game.hands());
    assert_eq!(parsed.dealer_card(), card(KING));
    assert_eq!(parsed.active(), 0);
    assert!(!parsed.is_split());
}

/// The bet is written with thousands separators, so the parse has to strip them
/// — a stake read as `1` would silently resolve the whole round at the wrong
/// stake rather than fail.
#[test]
fn a_formatted_stake_parses_back_to_the_same_number() {
    let emojis = emojis();

    for bet in [1, 999, 1_000, 1_234_567, i64::MAX] {
        let parsed = round_trip(&emojis, &opening_hand(bet)).unwrap();

        assert_eq!(parsed.bet(), bet, "stake {bet} did not survive the board");
    }
}

/// The case the marker character existed for. Both hands are on the board, and
/// which one is live has to come back — resuming on the wrong hand would deal
/// the player's card into the hand they already stood on.
#[test]
fn the_live_hand_is_recovered_after_a_split() {
    let emojis = emojis();

    // The shoe is only stocked by a parse, so round-trip once before splitting.
    let mut game = round_trip(
        &emojis,
        &GameDetails::new(500, vec![card(ACE), card(ACE)], card(KING)),
    )
    .unwrap();

    assert!(game.can_split(&emojis).unwrap(), "a pair should be splittable");

    game.split().expect("split should deal both hands");

    let first = round_trip(&emojis, &game).unwrap();

    assert!(first.is_split());
    assert_eq!(first.active(), 0, "play starts on the first hand");
    assert_eq!(first.hands(), game.hands());

    assert!(game.advance_hand(), "there should be a second hand to play");

    let second = round_trip(&emojis, &game).unwrap();

    assert!(second.is_split());
    assert_eq!(second.active(), 1, "the second hand must come back as the live one");
    assert_eq!(second.hands(), game.hands(), "hand order must be preserved");
}

/// Hands are read back by their badge index, not by the order the sections
/// happen to arrive in, so the two hands must not be interchangeable.
#[test]
fn split_hands_keep_their_order() {
    let emojis = emojis();

    let mut game = round_trip(
        &emojis,
        &GameDetails::new(100, vec![card(ACE), card(ACE)], card(KING)),
    )
    .unwrap();

    game.split().expect("split should deal both hands");

    let parsed = round_trip(&emojis, &game).unwrap();

    let (Some(original), Some(reparsed)) =
        (game.hands().first(), parsed.hands().first())
    else {
        panic!("both boards should carry a first hand");
    };

    assert_eq!(original, reparsed);
}

/// A board with no sections carries no round. Falling back to a default game
/// would resolve a phantom hand against the player's balance.
#[test]
fn an_empty_board_is_rejected_rather_than_defaulted() {
    let emojis = emojis();

    assert!(GameDetails::from_components(&emojis, &[]).is_err());
}
