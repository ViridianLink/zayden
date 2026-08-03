//! `/music clear mode:` parsing, which absorbed the deleted `removedupes` and
//! `cleanup` subcommands when slots were freed under Discord's 25-subcommand cap.
//!
//! A missing option must keep meaning "clear everything", because that was the
//! behaviour of the old bare `/music clear` and users' muscle memory relies on it.

use music::ClearMode;

#[test]
fn a_missing_mode_clears_the_whole_queue() {
    assert_eq!(ClearMode::parse(None), Some(ClearMode::All));
}

#[test]
fn each_choice_maps_to_its_operation() {
    assert_eq!(ClearMode::parse(Some("all")), Some(ClearMode::All));
    assert_eq!(ClearMode::parse(Some("duplicates")), Some(ClearMode::Duplicates));
    assert_eq!(ClearMode::parse(Some("left")), Some(ClearMode::Left));
}

#[test]
fn an_unknown_mode_is_rejected_rather_than_silently_clearing() {
    // Falling back to `All` on an unrecognised value would turn a typo (or a
    // stale client sending an old choice) into a full queue wipe.
    for mode in ["", "ALL", "dupes", "everything", "left "] {
        assert_eq!(
            ClearMode::parse(Some(mode)),
            None,
            "`{mode}` should be rejected"
        );
    }
}

#[test]
fn the_default_is_all() {
    assert_eq!(ClearMode::default(), ClearMode::All);
}
