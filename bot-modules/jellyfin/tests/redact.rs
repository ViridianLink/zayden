//! Plot redaction for `/jellyfin guess text`. Under-redacting gives the answer
//! away, so the rule errs towards redacting.

use jellyfin::games::question::redact::redact;

#[test]
fn the_title_is_always_removed() {
    let out = redact("Neo discovers the Matrix is a simulation.", "The Matrix");

    assert!(!out.contains("Matrix"), "the title leaked: {out}");
}

#[test]
fn character_names_are_removed() {
    let out = redact("A hacker named Neo meets Trinity in Zion.", "The Matrix");

    assert!(!out.contains("Neo"), "{out}");
    assert!(!out.contains("Trinity"), "{out}");
    assert!(!out.contains("Zion"), "{out}");
}

#[test]
fn ordinary_words_survive() {
    let out = redact("A hacker named Neo meets Trinity in Zion.", "The Matrix");

    assert!(out.contains("hacker"), "{out}");
    assert!(out.contains("meets"), "{out}");
}

#[test]
fn a_sentence_opening_word_is_not_treated_as_a_name() {
    let out = redact("The war changed everything.", "Some Film");

    assert!(out.starts_with("The war"), "{out}");
}

#[test]
fn empty_input_is_handled() {
    assert_eq!(redact("", "Anything"), "");
}
