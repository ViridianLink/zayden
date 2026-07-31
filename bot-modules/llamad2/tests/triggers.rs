//! Regression tests for the `llamad2` message triggers —
//! [CC-6](../../../design-docs/audits/_cross-cutting.md), the offline half of
//! the crate's coverage. No database: these pin the two predicates that decide
//! whether a handler acts at all.
//!
//! Most of the crate is a constant string sent to Discord (`/hello`,
//! `/socials`, `/playlist`, `/sensitivity`, `/raidreport`, `/dungeonreport`) —
//! deliberately left uncovered, since a test asserting a literal equals itself
//! is the trivia checklist #6 warns against. What *is* covered here is the
//! branching: the good-morning prefix match, its two-different-authors gate,
//! and the behind-the-scenes codeword match, which gates a **role grant**.
//!
//! **Mutation coverage** (each predicate broken in turn, suite re-run, then
//! reverted — these predicates were already correct, so this is what stands in
//! for fails-before):
//!
//! | Mutation | Result |
//! |---|---|
//! | `is_good_morning`: `starts_with` → `contains` | `the_greeting_match_is_a_prefix_of_the_trimmed_message` fails |
//! | `should_greet`: `last_author != author` dropped | `a_greeting_after_your_own_greeting_is_not_answered` fails |
//! | `is_codeword`: `eq_ignore_ascii_case` → `to_lowercase().contains` | `codewords_match_the_whole_message_case_insensitively` fails |

use llamad2::{is_codeword, is_good_morning, should_greet};
use serenity::all::UserId;

const ALICE: UserId = UserId::new(1);
const BOB: UserId = UserId::new(2);

/// The eight recognised greetings, each as the whole message.
#[test]
fn every_greeting_is_recognised() {
    for greeting in [
        "good morning",
        "gm",
        "goodmorning",
        "good mornin",
        "mornin",
        "morning",
        "g'mornin",
        "g morn",
    ] {
        assert!(is_good_morning(greeting), "{greeting:?} must be recognised");
    }
}

/// The match is on a **prefix** of the trimmed message, so a greeting followed
/// by more text still counts — and, as the flip side, a word that merely starts
/// with a greeting matches too.
///
/// Both halves are asserted so that changing `starts_with` to an exact match
/// (or to `contains`) fails here rather than silently changing which messages
/// the bot answers.
#[test]
fn the_greeting_match_is_a_prefix_of_the_trimmed_message() {
    assert!(is_good_morning("good morning everyone"), "trailing text is fine");
    assert!(is_good_morning("   gm   "), "surrounding whitespace is trimmed");

    // Known consequence of the prefix rule, pinned rather than fixed: the
    // alternation gate in `should_greet` is what keeps it from mattering.
    assert!(is_good_morning("gmail is down"), "the prefix rule matches this");

    assert!(!is_good_morning("say gm to everyone"), "not a prefix — no match");
    assert!(!is_good_morning("good evening"));
    assert!(!is_good_morning(""));
}

/// `is_good_morning` takes the content already lowercased by the caller, so it
/// is case-**sensitive** on its own.
///
/// Pins the split: the lowercasing lives in `GoodMorning::run`, and moving it
/// without moving this expectation would stop the bot answering "GM".
#[test]
fn the_greeting_match_expects_prelowercased_content() {
    assert!(is_good_morning("gm"));
    assert!(!is_good_morning("GM"), "the caller lowercases before calling in");
}

/// The bot answers only when two **different** people greet in a row.
#[test]
fn a_greeting_after_another_persons_greeting_is_answered() {
    assert!(should_greet(true, BOB, Some((ALICE, true))));
}

/// One person greeting twice must not trigger the reply — otherwise a single
/// user could spam it.
#[test]
fn a_greeting_after_your_own_greeting_is_not_answered() {
    assert!(!should_greet(true, ALICE, Some((ALICE, true))));
}

/// The previous message must itself have been a greeting, and this one must be
/// one too. Catches dropping either half of the condition.
#[test]
fn both_messages_must_be_greetings() {
    assert!(!should_greet(true, BOB, Some((ALICE, false))), "previous was chatter");
    assert!(!should_greet(false, BOB, Some((ALICE, true))), "this one is chatter");
    assert!(
        !should_greet(false, BOB, Some((ALICE, false))),
        "neither is a greeting"
    );
}

/// The first message in a channel has no predecessor cached, so nothing to
/// alternate with.
#[test]
fn the_first_message_in_a_channel_is_not_answered() {
    assert!(!should_greet(true, ALICE, None));
}

/// The seven codewords, each granting the behind-the-scenes role.
#[test]
fn every_codeword_is_recognised() {
    for code in
        ["password", "bonk", "fusion", "green man", "nova", "threadling", "buddy"]
    {
        assert!(is_codeword(code), "{code:?} must be recognised");
    }
}

/// Codewords are matched case-insensitively against the **whole** message.
///
/// The whole-message rule is the security-relevant half: this predicate gates
/// `add_member_role`, so loosening it to `contains` would grant the role to
/// anyone pasting a wordlist into the channel. Catches that change, and catches
/// swapping `eq_ignore_ascii_case` for `==`.
#[test]
fn codewords_match_the_whole_message_case_insensitively() {
    assert!(is_codeword("PASSWORD"), "case-insensitive");
    assert!(is_codeword("Green Man"), "…including multi-word codewords");

    assert!(!is_codeword("the password is bonk"), "must not match a substring");
    assert!(!is_codeword("password "), "…and is not trimmed");
    assert!(!is_codeword("passwords"));
    assert!(!is_codeword(""));
}
