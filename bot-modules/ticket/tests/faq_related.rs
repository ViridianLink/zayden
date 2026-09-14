//! Gating the related articles a solved ticket may add beside its primary one.
//!
//! A tangent only earns an article when code can confirm it was really solved,
//! because a weak related article is worse than none. Three checks carry that:
//!
//! - The confirmation must be the ticket author's own words in the conversation.
//!   Dropping the `user_said` check fails
//!   `a_helper_promising_a_fix_is_not_a_confirmation`.
//! - Every command, path, version and link must appear in the transcript. Dropping
//!   the literal check fails `an_invented_command_is_rejected`.
//! - At most `MAX_RELATED` are kept, and a rejected entry does not use up a slot.
//!   `rejected_entries_do_not_use_up_the_limit` fails if the count moves before
//!   vetting.

use ticket::faq::related::{MAX_RELATED, Rejection, RelatedEntry, sift, vet};
use ticket::faq::transcript::{MessageKind, RawMessage, render};
use ticket::faq::writer::WrittenArticle;

const AUTHOR: u64 = 1;
const HELPER: u64 = 2;

fn message(author_id: u64, kind: MessageKind, content: &str) -> RawMessage {
    RawMessage { author_id, kind, content: content.to_owned() }
}

fn transcript() -> Option<String> {
    let mut messages = vec![
        message(
            AUTHOR,
            MessageKind::TicketBody,
            "Issue: Radarr returns 502 behind nginx",
        ),
        message(
            HELPER,
            MessageKind::Human,
            "Run `docker restart nginx` and try again",
        ),
        message(AUTHOR, MessageKind::Human, "That fixed the 502, thank you"),
        message(
            AUTHOR,
            MessageKind::Human,
            "Sonarr also stopped importing to /data/tv",
        ),
        message(
            HELPER,
            MessageKind::Human,
            "Set `Root Folder` to /data/tv, that should do it",
        ),
        message(
            AUTHOR,
            MessageKind::Human,
            "Changing the root folder got imports working again",
        ),
    ];

    messages.reverse();
    render(Some("Radarr 502"), &messages, 10_000)
}

fn entry(title: &str, markdown: &str, confirmation: &str) -> RelatedEntry {
    RelatedEntry {
        article: WrittenArticle {
            title: title.to_owned(),
            summary: String::from("Imports stop after moving the library."),
            category: None,
            tags: Vec::new(),
            markdown: markdown.to_owned(),
        },
        confirmation: confirmation.to_owned(),
    }
}

fn good(title: &str) -> RelatedEntry {
    entry(
        title,
        "## Problem\nSonarr stops importing.\n## Solution\n1. Set `Root Folder` to `/data/tv`.",
        "got imports working again",
    )
}

#[test]
fn a_confirmed_grounded_entry_is_accepted() {
    assert_eq!(
        vet(
            &good("Sonarr not importing"),
            &transcript().expect("the thread has messages")
        ),
        Ok(())
    );
}

#[test]
fn a_helper_promising_a_fix_is_not_a_confirmation() {
    let entry = entry(
        "Sonarr not importing",
        "## Solution\n1. Set `Root Folder` to `/data/tv`.",
        "that should do it",
    );

    assert_eq!(
        vet(&entry, &transcript().expect("the thread has messages")),
        Err(Rejection::Unconfirmed)
    );
}

#[test]
fn a_bare_acknowledgement_is_not_a_confirmation() {
    let entry = entry("Sonarr not importing", "## Solution\n1. Restart.", "thank");

    assert_eq!(
        vet(&entry, &transcript().expect("the thread has messages")),
        Err(Rejection::Unconfirmed)
    );
}

#[test]
fn an_invented_command_is_rejected() {
    let entry = entry(
        "Sonarr not importing",
        "## Solution\n1. Run `chown -R 1000:1000 /data/tv`.",
        "got imports working again",
    );

    assert_eq!(
        vet(&entry, &transcript().expect("the thread has messages")),
        Err(Rejection::Ungrounded(vec![String::from(
            "chown -R 1000:1000 /data/tv"
        )]))
    );
}

#[test]
fn an_empty_article_is_rejected() {
    let entry = entry("Sonarr not importing", "", "got imports working again");

    assert_eq!(
        vet(&entry, &transcript().expect("the thread has messages")),
        Err(Rejection::Unusable)
    );
}

#[test]
fn entries_beyond_the_limit_are_rejected() {
    let entries = (0..=MAX_RELATED).map(|i| good(&format!("Entry {i}"))).collect();

    let sifted = sift(entries, &transcript().expect("the thread has messages"));

    assert_eq!(sifted.iter().filter(|result| result.is_ok()).count(), MAX_RELATED);
    assert!(matches!(
        sifted.last(),
        Some(Err(rejected)) if rejected.rejection == Rejection::OverLimit
    ));
}

#[test]
fn rejected_entries_do_not_use_up_the_limit() {
    let mut entries =
        vec![entry("Unconfirmed", "## Solution\n1. Restart.", "nothing")];
    entries.extend((0..MAX_RELATED).map(|i| good(&format!("Entry {i}"))));

    let accepted = sift(entries, &transcript().expect("the thread has messages"))
        .into_iter()
        .filter_map(Result::ok)
        .map(|article| article.title)
        .collect::<Vec<_>>();

    assert_eq!(accepted.len(), MAX_RELATED);
    assert!(!accepted.contains(&String::from("Unconfirmed")));
}
