//! What the keyword extractor searches the wiki from.
//!
//! The title is always part of it: a forum post's title is often the only
//! summary of the problem, and a screenshot-only post has nothing else.

use ticket::faq::keywords::query;

#[test]
fn the_title_is_always_searched() {
    let query = query("Palworld server crashes on join", "it just dies", "");

    assert!(query.contains("Palworld server crashes on join"), "{query}");
    assert!(query.contains("it just dies"), "{query}");
}

#[test]
fn screenshot_text_is_searched() {
    let query =
        query("Crash", "", "Image 1: java.net.BindException: Address in use");

    assert!(query.contains("BindException"), "{query}");
}

#[test]
fn a_title_only_ticket_still_has_something_to_search() {
    assert_eq!(
        query("Server will not start", "  ", ""),
        "Title:\nServer will not start"
    );
}

#[test]
fn empty_sections_are_left_out() {
    let query = query("Crash", "", "");

    assert!(!query.contains("Message:"), "{query}");
    assert!(!query.contains("Screenshots:"), "{query}");
}
