//! Autocomplete choice encoding. Discord caps a choice's name and value at 100
//! characters and rejects the whole response if either is over.

use ticket::faq::Target;

#[test]
fn a_page_target_round_trips() {
    let target = Target { id: 42, anchor: None };

    assert_eq!(Target::parse(&target.value()), Some(target));
}

#[test]
fn a_heading_target_round_trips() {
    let target = Target { id: 42, anchor: Some(String::from("backups")) };

    assert_eq!(Target::parse(&target.value()), Some(target));
}

/// Anything without the sentinel is a question the user typed, and questions
/// must not be mistaken for page references.
#[test]
fn a_typed_question_is_not_a_target() {
    assert!(Target::parse("how do I restore a backup").is_none());
    assert!(Target::parse("faq://page/notanumber").is_none());
    assert!(Target::parse("").is_none());
}

#[test]
fn a_value_stays_within_discords_cap() {
    let target = Target { id: 1234, anchor: Some("section-title-".repeat(20)) };

    assert!(target.value().chars().count() <= 100, "{}", target.value());
}
