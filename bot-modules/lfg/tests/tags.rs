//! Regression coverage for the `/lfg tags` select-menu construction.
//!
//! Audit finding ticket DS-2: when the tag filter leaves nothing selectable
//! (a thread already carrying every available tag for `add`, or with none
//! applied for `remove`), the command used to build a string select menu with
//! zero options and `max_values(0)`, which Discord rejects with a 400. The fix
//! routes the empty case to a friendly notice instead, so the invariant under
//! test is: `build_tag_response` returns `Notice` in exactly those cases and a
//! `Menu` with only the selectable tags otherwise.

use lfg::commands::tags::{TagAction, TagResponse, build_tag_response};
use serenity::all::ForumTagId;

fn available() -> Vec<(ForumTagId, String)> {
    vec![
        (ForumTagId::new(1), "PvP".to_string()),
        (ForumTagId::new(2), "PvE".to_string()),
        (ForumTagId::new(3), "Raid".to_string()),
    ]
}

fn menu_len(response: TagResponse) -> Option<usize> {
    match response {
        TagResponse::Notice(_) => None,
        TagResponse::Menu(options) => Some(options.len()),
    }
}

#[test]
fn add_on_fully_tagged_thread_is_a_notice() {
    let applied = [ForumTagId::new(1), ForumTagId::new(2), ForumTagId::new(3)];

    // Every available tag is already applied → nothing left to add, so the
    // response must be a notice, not a zero-option menu (the 400 the fix guards).
    let response = build_tag_response(available(), &applied, TagAction::Add);
    assert!(
        matches!(response, TagResponse::Notice(_)),
        "add on a fully-tagged thread must be a notice, not a menu",
    );
}

#[test]
fn add_offers_only_unapplied_tags() {
    let applied = [ForumTagId::new(1)];

    let response = build_tag_response(available(), &applied, TagAction::Add);
    assert_eq!(menu_len(response), Some(2), "add must offer the two not-yet-applied tags");
}

#[test]
fn remove_on_untagged_thread_is_a_notice() {
    // No tags applied → nothing to remove.
    let response = build_tag_response(available(), &[], TagAction::Remove);
    assert!(
        matches!(response, TagResponse::Notice(_)),
        "remove on an untagged thread must be a notice, not a menu",
    );
}

#[test]
fn remove_offers_only_applied_tags() {
    let applied = [ForumTagId::new(2), ForumTagId::new(3)];

    let response = build_tag_response(available(), &applied, TagAction::Remove);
    assert_eq!(menu_len(response), Some(2), "remove must offer only the applied tags");
}
