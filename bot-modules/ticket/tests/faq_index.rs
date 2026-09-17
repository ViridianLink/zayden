//! Autocomplete choice encoding. Discord caps a choice's name and value at 100
//! characters and rejects the whole response if either is over.

use serenity::all::{AutocompleteChoice, AutocompleteValue};
use ticket::faq::Target;
use ticket::faq::index::choice::ask;

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

/// The typed question is the first row, so pressing enter sends it instead of
/// whichever page happened to rank first.
#[test]
fn a_typed_question_is_offered_verbatim() {
    let choice = ask("how do I restore a backup");

    assert_eq!(
        value(choice.as_ref()),
        Some(String::from("how do I restore a backup"))
    );
}

/// Discord rejects a choice with an empty value and drops the rest of the
/// response with it, so an untouched box offers pages only.
#[test]
fn an_empty_query_is_not_offered() {
    assert!(ask("").is_none());
    assert!(ask("   ").is_none());
}

/// The cap applies to the value as well, and a cut question reads better
/// ending on a whole word than mid-word.
#[test]
fn a_long_question_is_cut_at_a_word() {
    let query = "how do I restore a backup ".repeat(10);

    let value = value(ask(&query).as_ref()).unwrap_or_default();
    let cut = value.chars().count();

    assert!((1..=100).contains(&cut), "{value}");
    assert!(query.starts_with(&value), "{value}");
    assert_eq!(query.chars().nth(cut), Some(' '), "{value}");
}

fn value(choice: Option<&AutocompleteChoice<'_>>) -> Option<String> {
    match choice?.value {
        AutocompleteValue::String(ref value) => Some(value.to_string()),
        AutocompleteValue::Integer(_) | AutocompleteValue::Float(_) | _ => None,
    }
}
