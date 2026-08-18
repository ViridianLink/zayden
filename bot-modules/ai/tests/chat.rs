use ai::chat::strip_speaker_prefix;

/// The reported bug: asked "who are you?", the model mirrored the `Name: ...`
/// transcript format and addressed its own reply to the person who asked.
#[test]
fn strips_the_asker_name_the_model_mirrored_back() {
    let speakers = ["Zayden", "OscarSix"];

    assert_eq!(
        strip_speaker_prefix(
            "OscarSix: Just Zayden. The one who gets things done.",
            &speakers
        ),
        "Just Zayden. The one who gets things done."
    );
}

#[test]
fn strips_the_personas_own_signature() {
    assert_eq!(
        strip_speaker_prefix("Zayden: Ask me something and find out.", &[
            "Zayden", "OscarSix"
        ]),
        "Ask me something and find out."
    );
}

#[test]
fn strips_a_name_whose_case_does_not_match() {
    assert_eq!(
        strip_speaker_prefix("oscarsix: hey", &["Zayden", "OscarSix"]),
        "hey"
    );
}

#[test]
fn strips_a_signature_stacked_on_another() {
    assert_eq!(
        strip_speaker_prefix("Zayden: OscarSix: hey", &["Zayden", "OscarSix"]),
        "hey"
    );
}

#[test]
fn strips_a_signature_left_on_its_own_line() {
    assert_eq!(
        strip_speaker_prefix("Zayden:\nHey.", &["Zayden", "OscarSix"]),
        "Hey."
    );
}

#[test]
fn keeps_a_colon_that_is_not_a_signature() {
    let speakers = ["Zayden", "OscarSix"];

    assert_eq!(
        strip_speaker_prefix("Rule one: don't panic.", &speakers),
        "Rule one: don't panic."
    );
    assert_eq!(
        strip_speaker_prefix("Viktor: he'd say something worse.", &speakers),
        "Viktor: he'd say something worse."
    );
}

/// A name-shaped opening only counts as a signature when the colon follows the
/// name exactly, not when the name merely starts a longer word or sentence.
#[test]
fn keeps_a_speaker_name_used_mid_sentence() {
    let speakers = ["Zayden", "OscarSix"];

    assert_eq!(
        strip_speaker_prefix("Zayden is the name.", &speakers),
        "Zayden is the name."
    );
    assert_eq!(
        strip_speaker_prefix("OscarSixty asked: fair question.", &speakers),
        "OscarSixty asked: fair question."
    );
}

/// Stripping a bare signature would leave nothing to send, and Discord rejects
/// an empty message.
#[test]
fn keeps_a_reply_that_is_nothing_but_a_signature() {
    assert_eq!(strip_speaker_prefix("Zayden:", &["Zayden"]), "Zayden:");
    assert_eq!(
        strip_speaker_prefix("OscarSix: Zayden:", &["Zayden", "OscarSix"]),
        "Zayden:"
    );
}

#[test]
fn trims_surrounding_whitespace_with_no_speakers_to_match() {
    assert_eq!(strip_speaker_prefix("  hey  ", &[]), "hey");
}

/// An empty display name must not swallow the reply, or match forever.
#[test]
fn ignores_an_empty_speaker_name() {
    assert_eq!(strip_speaker_prefix(": hey", &[""]), ": hey");
}

/// Byte-indexing a multi-byte name must not panic or half-match.
#[test]
fn handles_multi_byte_speaker_names() {
    assert_eq!(strip_speaker_prefix("Óscar: hey", &["Óscar"]), "hey");
    assert_eq!(strip_speaker_prefix("Ósc: hey", &["Óscar"]), "Ósc: hey");
}
