//! The title fallback for support channels that cannot hold a forum tag.
//!
//! A ticket's state is shown as a forum tag wherever possible. A thread under a
//! plain text channel has nowhere to put one, and neither does a forum whose
//! tag is unset or no longer offered, so those fall back to prefixing the
//! title — the behaviour that predates tag support.
//!
//! The prefix has to *replace* whatever state the title already carries.
//! `/ticket close` on an already-fixed post previously produced
//! "[Closed] - [Fixed] - name", and `/ticket open` only stripped two of the
//! three prefixes, so a solved post could never be reopened by name.
//!
//! "[Fixed] - " is no longer applied — `/ticket fixed` is gone — but threads
//! titled before its removal still carry it, so it stays strippable.

use ticket::state::{CLOSED, SOLVED, retitle};

const LEGACY_FIXED: &str = "[Fixed] - ";

#[test]
fn a_fresh_ticket_takes_the_prefix() {
    assert_eq!(
        retitle("1 - reporter - mod menu", SOLVED),
        "[Solved] - 1 - reporter - mod menu"
    );
}

#[test]
fn a_state_prefix_replaces_the_previous_one() {
    let solved = retitle("1 - reporter - mod menu", SOLVED);

    assert_eq!(retitle(&solved, CLOSED), "[Closed] - 1 - reporter - mod menu");
}

/// A thread left "[Fixed] - " by the retired command must still be closable
/// and reopenable without the prefix stacking.
#[test]
fn a_legacy_fixed_prefix_is_still_replaced() {
    let legacy = format!("{LEGACY_FIXED}1 - reporter - mod menu");

    assert_eq!(retitle(&legacy, CLOSED), "[Closed] - 1 - reporter - mod menu");
    assert_eq!(retitle(&legacy, ""), "1 - reporter - mod menu");
}

/// The regression: `open` used to `.replace()` only the fixed and closed
/// prefixes, so a solved post kept its prefix forever.
#[test]
fn reopening_strips_every_state_prefix() {
    for prefix in [SOLVED, CLOSED, LEGACY_FIXED] {
        let marked = retitle("1 - reporter - mod menu", prefix);

        assert_eq!(retitle(&marked, ""), "1 - reporter - mod menu");
    }
}

#[test]
fn reopening_an_already_open_ticket_leaves_the_title_alone() {
    assert_eq!(retitle("1 - reporter - mod menu", ""), "1 - reporter - mod menu");
}

/// Re-running the same command must not stack the prefix either.
#[test]
fn repeating_a_command_is_idempotent() {
    let once = retitle("1 - reporter - mod menu", CLOSED);

    assert_eq!(retitle(&once, CLOSED), once);
}

/// Discord rejects a thread name over 100 characters.
#[test]
fn the_title_is_capped_at_discords_limit() {
    let long = "x".repeat(200);

    let marked = retitle(&long, SOLVED);

    assert_eq!(marked.chars().count(), 100);
    assert!(marked.starts_with(SOLVED));
}

/// A prefix only counts at the start; one quoted mid-title is left as text.
#[test]
fn a_prefix_inside_the_title_is_not_stripped() {
    let name = "1 - reporter - why does it say [Closed] - here";

    assert_eq!(retitle(name, ""), name);
}
