//! Regression tests for [CC-7](../../../design-docs/audits/_cross-cutting.md) —
//! component `custom_id` string routing in `gambling`.
//!
//! CC-7 was that four interaction handlers routed on
//! `match interaction.data.custom_id.as_str()` against bare string literals,
//! with the matching literals written out a second time at the
//! `CreateButton::new(...)` call site. Nothing tied producer to consumer, so a
//! typo in either half compiled clean and silently dead-ended the button.
//!
//! The concrete defect that stringly routing permitted lived in
//! `components/tictactoe.rs`: the cancel arm carried an ownership guard
//! (`"ttt_cancel" if metadata.user == interaction.user`), and a match guard that
//! fails falls through to the *next* arm rather than out of the match. A
//! non-owner's cancel click therefore reached the catch-all, which stripped the
//! `ttt_` prefix and tried to read `"cancel"` as board coordinates — surfacing
//! as `Internal("row index not parseable")`, which per `Respond` has no
//! `user_message`, so the clicker got a generic failure instead of being told
//! it was not their game. Parsing to a typed variant *first* and applying the
//! ownership check *inside* the arm makes that fall-through unrepresentable.
//!
//! Routing itself needs a live `ComponentInteraction` and `Http`, for which this
//! crate has no harness (audit CC-6). What is testable in-process is the
//! producer/consumer contract the fix introduces: the enums are now the single
//! source of both halves, so pinning `as_str` ↔ `FromStr` here covers the drift
//! CC-7 describes.

use std::str::FromStr;

use gambling::components::{
    BlackjackCustomId,
    HandState,
    HigherLowerCustomId,
    PrestigeCustomId,
    TicTacToeCustomId,
};

/// The CC-7 defect proper: `ttt_cancel` must resolve to the cancel variant.
///
/// Under the old `match` this string reached the coordinate-parsing catch-all
/// whenever the ownership guard failed. A variant cannot fall through, so the
/// non-owner path is now an ownership decision inside `Cancel` rather than a
/// misparse.
#[test]
fn ttt_cancel_parses_as_cancel_and_never_as_a_board_cell() {
    let id = TicTacToeCustomId::from_str("ttt_cancel").unwrap();

    assert_eq!(id, TicTacToeCustomId::Cancel);
    assert!(
        !matches!(id, TicTacToeCustomId::Cell { .. }),
        "`ttt_cancel` must not be readable as coordinates"
    );
}

/// `ttt_accept` is the other unit variant sharing the `ttt_` prefix with cells.
#[test]
fn ttt_accept_parses_as_accept_and_never_as_a_board_cell() {
    let id = TicTacToeCustomId::from_str("ttt_accept").unwrap();

    assert_eq!(id, TicTacToeCustomId::Accept);
    assert!(!matches!(id, TicTacToeCustomId::Cell { .. }));
}

/// Board buttons keep their `ttt_{row}{col}` shape, so messages posted before
/// this change still route.
#[test]
fn board_cells_round_trip_through_their_rendered_id() {
    for row in 0..5 {
        for col in 0..5 {
            let id = TicTacToeCustomId::Cell { row, col };

            assert_eq!(id.to_string(), format!("ttt_{row}{col}"));
            assert_eq!(TicTacToeCustomId::from_str(&id.to_string()).unwrap(), id);
        }
    }
}

/// A malformed coordinate is an error, not a silently-clamped move.
#[test]
fn malformed_tictactoe_ids_are_rejected() {
    for bad in ["ttt_", "ttt_a1", "ttt_1a", "ttt_1", "ttt_123", "prestige_cancel"] {
        assert!(
            TicTacToeCustomId::from_str(bad).is_err(),
            "`{bad}` should not parse as a tictactoe component id"
        );
    }
}

/// Producer/consumer agreement for the three unit-only enums. These ids are
/// live in Discord on already-posted messages, so the literals are pinned, not
/// just round-tripped.
#[test]
fn unit_variants_round_trip_and_keep_their_wire_ids() {
    let blackjack = [
        (BlackjackCustomId::Hit, "blackjack_hit"),
        (BlackjackCustomId::Stand, "blackjack_stand"),
        (BlackjackCustomId::Double, "blackjack_double"),
        (BlackjackCustomId::Split, "blackjack_split"),
        (BlackjackCustomId::Surrender, "blackjack_surrender"),
    ];

    for (variant, wire) in blackjack {
        assert_eq!(variant.to_string(), wire);
        assert_eq!(BlackjackCustomId::from_str(wire).unwrap(), variant);
    }

    let higher_lower = [
        (HigherLowerCustomId::Higher, "hol_higher"),
        (HigherLowerCustomId::Lower, "hol_lower"),
    ];

    for (variant, wire) in higher_lower {
        assert_eq!(variant.as_str(), wire);
        assert_eq!(HigherLowerCustomId::from_str(wire).unwrap(), variant);
    }

    let prestige = [
        (PrestigeCustomId::Confirm, "prestige_confirm"),
        (PrestigeCustomId::Cancel, "prestige_cancel"),
    ];

    for (variant, wire) in prestige {
        assert_eq!(variant.as_str(), wire);
        assert_eq!(PrestigeCustomId::from_str(wire).unwrap(), variant);
    }
}

/// Each enum owns only its own namespace — the bot's `IdMatch::Prefix` routing
/// dispatches by prefix, so a cross-namespace id must not parse.
#[test]
fn ids_do_not_cross_namespaces() {
    assert!(BlackjackCustomId::from_str("hol_higher").is_err());
    assert!(HigherLowerCustomId::from_str("blackjack_hit").is_err());
    assert!(PrestigeCustomId::from_str("ttt_cancel").is_err());
    assert!(TicTacToeCustomId::from_str("blackjack_split").is_err());

    assert!(BlackjackCustomId::from_str("").is_err());
    assert!(HigherLowerCustomId::from_str("hol_").is_err());
    assert!(PrestigeCustomId::from_str("prestige").is_err());
}

/// The blackjack board marks the live hand with a badge in each section's
/// accessory slot rather than with a marker character in the heading, so these
/// ids *are* the round's state. A hand whose badge does not round-trip is a hand
/// the next button press cannot find.
#[test]
fn hand_badge_ids_round_trip() {
    let badges = [
        (
            BlackjackCustomId::Hand { index: 0, state: HandState::Active },
            "blackjack_hand_0_active",
        ),
        (
            BlackjackCustomId::Hand { index: 1, state: HandState::Waiting },
            "blackjack_hand_1_waiting",
        ),
        (
            BlackjackCustomId::Hand { index: 1, state: HandState::Done },
            "blackjack_hand_1_done",
        ),
        (
            BlackjackCustomId::Dealer { state: HandState::Waiting },
            "blackjack_dealer_waiting",
        ),
        (
            BlackjackCustomId::Dealer { state: HandState::Done },
            "blackjack_dealer_done",
        ),
    ];

    for (variant, wire) in badges {
        assert_eq!(variant.to_string(), wire);
        assert_eq!(BlackjackCustomId::from_str(wire).unwrap(), variant);
    }
}

/// Discord requires every custom id in a message to be unique, so two hands on
/// the same board must not collide — which is the whole reason the index is in
/// the id rather than the state alone.
#[test]
fn hand_badges_on_one_board_stay_distinct() {
    let split_board = [
        BlackjackCustomId::Hand { index: 0, state: HandState::Done },
        BlackjackCustomId::Hand { index: 1, state: HandState::Done },
        BlackjackCustomId::Dealer { state: HandState::Done },
    ]
    .map(|id| id.to_string());

    let mut unique = split_board.to_vec();
    unique.sort_unstable();
    unique.dedup();

    assert_eq!(
        unique.len(),
        split_board.len(),
        "duplicate custom id on one board: {split_board:?}"
    );
}

/// A badge id has to survive the same namespace guard as the action buttons —
/// it is disabled, but the router still matches on the `blackjack` prefix.
#[test]
fn malformed_badge_ids_do_not_parse() {
    for bad in [
        "blackjack_hand_0",
        "blackjack_hand__active",
        "blackjack_hand_x_active",
        "blackjack_hand_0_playing",
        "blackjack_dealer",
        "blackjack_dealer_x",
        "blackjack_hand_999999_active",
    ] {
        assert!(
            BlackjackCustomId::from_str(bad).is_err(),
            "`{bad}` should not parse as a blackjack component id"
        );
    }
}
