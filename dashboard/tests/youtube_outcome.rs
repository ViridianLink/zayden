//! The OAuth handler and the settings tab agree on the redirect's `?youtube=`
//! value through `YoutubeOutcome` alone: the handler writes `as_key`, the tab
//! reads `from_key`. Two variants sharing a key would silently rewrite one
//! outcome into the other, and nothing else in the build would notice.
#![cfg(feature = "ssr")]

use std::collections::BTreeSet;

use dashboard::dto::YoutubeOutcome;

#[test]
fn every_outcome_has_its_own_key() {
    let keys: BTreeSet<_> =
        YoutubeOutcome::ALL.iter().map(|outcome| outcome.as_key()).collect();

    assert_eq!(keys.len(), YoutubeOutcome::ALL.len());
}

#[test]
fn every_key_parses_back_to_the_outcome_that_wrote_it() {
    for outcome in YoutubeOutcome::ALL {
        assert_eq!(YoutubeOutcome::from_key(outcome.as_key()), Some(outcome));
    }
}

#[test]
fn an_unrecognised_value_renders_no_banner() {
    assert_eq!(YoutubeOutcome::from_key(""), None);
    assert_eq!(YoutubeOutcome::from_key("connect"), None);
    assert_eq!(YoutubeOutcome::from_key("Connected"), None);
}

/// A copy-pasted arm would explain the wrong failure to a creator.
#[test]
fn no_two_outcomes_explain_themselves_the_same_way() {
    let messages: BTreeSet<_> =
        YoutubeOutcome::ALL.iter().map(|outcome| outcome.message()).collect();

    assert_eq!(messages.len(), YoutubeOutcome::ALL.len());
}

/// The banner is only visible if the class it carries has a rule; every other
/// severity in the product comes from this same partial.
#[test]
fn every_severity_class_is_defined_in_the_stylesheet() {
    const FEEDBACK: &str = include_str!("../style/partials/pages.css");

    for outcome in YoutubeOutcome::ALL {
        let rule = format!(".{} {{", outcome.class());
        assert!(FEEDBACK.contains(&rule), "{rule} is not defined");
    }
}

/// The banner is rendered into an ARIA live region, and only a real failure is
/// allowed to interrupt: `role="alert"` is assertive by definition.
#[test]
fn only_the_failures_interrupt_the_reader() {
    for outcome in YoutubeOutcome::ALL {
        let expected = if outcome.class() == "error" { "alert" } else { "status" };

        assert_eq!(outcome.role(), expected, "{outcome:?}");
    }
}
