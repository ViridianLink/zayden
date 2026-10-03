//! The shared strict fold behind every dashboard form.

use web::engagement::EngagementError;
use web::form::{FieldError, fold};
use web::guild::GuildError;

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

#[test]
fn values_come_back_in_the_order_the_names_were_given() {
    let [a, b, c] =
        fold(pairs(&[("c", "3"), ("a", " 1 "), ("b", "")]), ["a", "b", "c"])
            .unwrap();

    assert_eq!((a.as_str(), b.as_str(), c.as_str()), (" 1 ", "", "3"));
}

#[test]
fn an_unknown_repeated_or_missing_field_is_refused() {
    assert_eq!(
        fold(pairs(&[("a", "1"), ("x", "2")]), ["a"]),
        Err(FieldError::Unknown("x".to_owned()))
    );
    assert_eq!(
        fold(pairs(&[("a", "1"), ("a", "2")]), ["a"]),
        Err(FieldError::Duplicate("a".to_owned()))
    );
    assert_eq!(
        fold(pairs(&[("a", "1")]), ["a", "b"]),
        Err(FieldError::Missing("b"))
    );
    assert_eq!(fold(Vec::new(), ["a", "b"]), Err(FieldError::Missing("a")));
}

#[test]
fn the_first_fault_in_submission_order_wins_over_a_missing_field() {
    assert_eq!(
        fold(pairs(&[("x", "1")]), ["a"]),
        Err(FieldError::Unknown("x".to_owned()))
    );
}

#[test]
fn field_errors_read_the_same_everywhere() {
    let cases = [
        (FieldError::Unknown("x".to_owned()), "unknown field `x`"),
        (FieldError::Duplicate("x".to_owned()), "duplicate field `x`"),
        (FieldError::Missing("x"), "missing field `x`"),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
        assert_eq!(GuildError::from(error.clone()).to_string(), message);
        assert_eq!(EngagementError::from(error).to_string(), message);
    }
}

#[test]
fn field_errors_convert_to_the_matching_variants() {
    assert_eq!(
        GuildError::from(FieldError::Unknown("x".to_owned())),
        GuildError::UnknownField("x".to_owned())
    );
    assert_eq!(
        GuildError::from(FieldError::Duplicate("x".to_owned())),
        GuildError::DuplicateField("x".to_owned())
    );
    assert_eq!(
        GuildError::from(FieldError::Missing("x")),
        GuildError::MissingField("x")
    );
    assert_eq!(
        EngagementError::from(FieldError::Unknown("x".to_owned())),
        EngagementError::UnknownField("x".to_owned())
    );
    assert_eq!(
        EngagementError::from(FieldError::Duplicate("x".to_owned())),
        EngagementError::DuplicateField("x".to_owned())
    );
    assert_eq!(
        EngagementError::from(FieldError::Missing("x")),
        EngagementError::MissingField("x")
    );
    assert!(GuildError::from(FieldError::Missing("x")).is_invalid_form());
    assert!(EngagementError::from(FieldError::Missing("x")).is_invalid_form());
}
