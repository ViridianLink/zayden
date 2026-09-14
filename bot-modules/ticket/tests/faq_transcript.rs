//! Turning a support thread into the writer prompt's transcript.
//!
//! Five rules are load-bearing and none can be checked against a live Discord
//! thread in CI, so the cleaning runs over a plain message list that the Discord
//! side builds first.
//!
//! - The opening problem statement is kept. Neither creation path leaves it as a
//!   human message: `message_command.rs` deletes the original and reposts it as an
//!   "Issue" embed, and `modal.rs` posts one embed per modal field. Treating every
//!   bot message as noise dropped the only statement of what was actually wrong, and
//!   `the_ticket_body_is_kept` fails without the `TicketBody` arm.
//! - The author is recovered from the opening message's mention, not from who speaks
//!   first. A helper often replies before the author does, so
//!   `the_author_is_taken_from_the_opening_mention` fails if `render` falls back to
//!   first-to-speak.
//! - Speakers are relabelled, not named, so no username can reach the provider or
//!   the published article.
//! - Truncation keeps the original issue and the END of the conversation. The writer
//!   is told to answer the original issue, so losing it on a long thread hands it
//!   whatever tangent the thread ended on, and a support thread reaches its solution
//!   last. `truncation_keeps_the_opening_and_the_end` fails if the opening is
//!   truncated with the conversation or `tail` is reversed.
//! - `user_said` only accepts the ticket author's words from the conversation. The
//!   related-article gate relies on it to prove the user confirmed a fix, so a
//!   helper's "that should fix it" or the opening post must not count.

use ticket::faq::transcript::{MessageKind, RawMessage, render, user_said};

const LIMIT: usize = 10_000;
const AUTHOR: u64 = 1;
const HELPER: u64 = 2;

fn human(author_id: u64, content: &str) -> RawMessage {
    RawMessage { author_id, kind: MessageKind::Human, content: content.to_owned() }
}

fn body(author_id: u64, content: &str) -> RawMessage {
    RawMessage {
        author_id,
        kind: MessageKind::TicketBody,
        content: content.to_owned(),
    }
}

fn triage(content: &str) -> RawMessage {
    RawMessage {
        author_id: 0,
        kind: MessageKind::Triage,
        content: content.to_owned(),
    }
}

/// `render` takes messages newest first, the order Discord returns them in.
fn thread_with(
    title: Option<&str>,
    messages: Vec<RawMessage>,
    limit: usize,
) -> Option<String> {
    let mut messages = messages;
    messages.reverse();
    render(title, &messages, limit)
}

fn thread(messages: Vec<RawMessage>) -> Option<String> {
    thread_with(None, messages, LIMIT)
}

fn conversation(rendered: &str) -> &str {
    rendered
        .split_once("\n\nConversation:\n")
        .map_or("", |(_, conversation)| conversation)
}

#[test]
fn an_empty_thread_produces_nothing() {
    assert_eq!(render(Some("Radarr 502"), &[], LIMIT), None);
}

#[test]
fn a_thread_of_only_chatter_produces_nothing() {
    assert_eq!(thread(vec![human(AUTHOR, "hi"), human(HELPER, "yo")]), None);
}

#[test]
fn the_ticket_body_is_kept() {
    let rendered = thread(vec![
        body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
        human(HELPER, "Have you restarted the reverse proxy yet?"),
    ])
    .expect("the ticket body is always kept");

    assert!(
        rendered
            .starts_with("Original issue:\nUser: Issue: Radarr is throwing a 502")
    );
}

#[test]
fn a_short_ticket_body_is_still_kept() {
    let rendered = thread(vec![body(AUTHOR, "502s")])
        .expect("the ticket body bypasses the length filter");

    assert_eq!(rendered, "Original issue:\nUser: 502s");
}

#[test]
fn the_thread_title_opens_the_original_issue() {
    let rendered = thread_with(
        Some("Radarr 502 behind nginx"),
        vec![
            body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
            human(HELPER, "Have you restarted the reverse proxy yet?"),
        ],
        LIMIT,
    )
    .expect("the thread has usable messages");

    assert!(rendered.starts_with(
        "Original issue:\nThread title: Radarr 502 behind nginx\nUser: Issue:"
    ));
    assert_eq!(
        conversation(&rendered),
        "Helper 1: Have you restarted the reverse proxy yet?"
    );
}

#[test]
fn the_triage_questions_belong_to_the_conversation() {
    let rendered = thread(vec![
        body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
        triage("Diagnostic questions asked:\n1. Which version?"),
        human(AUTHOR, "I am on version 5.2 of Radarr behind Caddy"),
    ])
    .expect("the thread has usable messages");

    let conversation = conversation(&rendered);

    assert!(conversation.starts_with("Support Bot: Diagnostic questions asked:"));
    assert!(conversation.contains("1. Which version?"));
}

#[test]
fn the_author_is_taken_from_the_opening_mention() {
    // The helper speaks before the author does, which is the common case on the
    // message creation path: the author's own message was deleted.
    let rendered = thread(vec![
        body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
        human(HELPER, "Have you restarted the reverse proxy yet?"),
        human(AUTHOR, "Yes, I restarted it twice with no change"),
    ])
    .expect("the thread has usable messages");

    assert!(rendered.contains("Helper 1: Have you restarted"));
    assert!(rendered.contains("User: Yes, I restarted it twice"));
}

#[test]
fn without_a_ticket_body_the_first_message_is_the_original_issue() {
    let rendered = thread(vec![
        human(AUTHOR, "Radarr is throwing a 502 error again"),
        human(HELPER, "Have you restarted the reverse proxy yet?"),
    ])
    .expect("the thread has two long enough messages");

    assert!(rendered.starts_with("Original issue:\nUser: Radarr"));
    assert_eq!(
        conversation(&rendered),
        "Helper 1: Have you restarted the reverse proxy yet?"
    );
}

#[test]
fn helpers_are_numbered_in_the_order_they_speak() {
    let rendered = thread(vec![
        body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
        human(7, "Which reverse proxy are you running?"),
        human(4, "Check the container logs first please"),
        human(7, "Post the output of docker compose ps"),
    ])
    .expect("every message is long enough");

    assert!(rendered.contains("Helper 1: Which reverse proxy"));
    assert!(rendered.contains("Helper 2: Check the container"));
    assert!(rendered.contains("Helper 1: Post the output"));
}

#[test]
fn every_speaker_is_a_label_not_a_name() {
    // Single-line content only: a message body's continuation lines carry no
    // label of their own, which `the_triage_questions_belong_to_the_conversation`
    // covers instead.
    let rendered = thread_with(
        Some("Radarr 502"),
        vec![
            body(AUTHOR, "Issue: Radarr is throwing a 502 error again"),
            triage("Diagnostic questions asked"),
            human(HELPER, "Have you restarted the reverse proxy yet?"),
        ],
        LIMIT,
    )
    .expect("the thread has usable messages");

    for line in rendered.lines().filter(|line| {
        !line.is_empty() && *line != "Original issue:" && *line != "Conversation:"
    }) {
        let speaker = line.split(':').next().unwrap_or_default();

        assert!(
            speaker == "User"
                || speaker == "Support Bot"
                || speaker == "Thread title"
                || speaker.starts_with("Helper"),
            "unexpected speaker label: {speaker}"
        );
    }
}

#[test]
fn short_chatter_is_dropped() {
    let rendered = thread(vec![
        human(AUTHOR, "Radarr is throwing a 502 error again"),
        human(HELPER, "thanks!"),
        human(HELPER, "Restart the reverse proxy and try once more"),
    ])
    .expect("two messages are long enough");

    assert!(!rendered.contains("thanks!"));
    assert_eq!(conversation(&rendered).lines().count(), 1);
}

#[test]
fn truncation_keeps_the_opening_and_the_end() {
    let mut messages =
        vec![body(AUTHOR, "Issue: Radarr is throwing a 502 error again")];

    for i in 0..50 {
        messages.push(human(AUTHOR, &format!("message number {i} with padding")));
    }

    messages.push(human(HELPER, "The fix was to restart the reverse proxy"));

    let trimmed =
        thread_with(None, messages, 200).expect("the thread has usable messages");

    assert!(trimmed.starts_with("Original issue:\nUser: Issue: Radarr"));
    assert!(trimmed.ends_with("Helper 1: The fix was to restart the reverse proxy"));
    assert!(!trimmed.contains("message number 0 "));
    assert!(trimmed.chars().count() <= 200);
}

#[test]
fn a_long_opening_cannot_crowd_out_the_conversation() {
    let opening = "The container keeps restarting.\n\n".repeat(20);

    let trimmed = thread_with(
        None,
        vec![
            body(AUTHOR, &opening),
            human(HELPER, "The fix was to restart the reverse proxy"),
        ],
        300,
    )
    .expect("the thread has usable messages");

    assert!(trimmed.contains("[truncated]"));
    assert!(trimmed.ends_with("Helper 1: The fix was to restart the reverse proxy"));
}

#[test]
fn truncation_never_splits_a_line() {
    let rendered = thread_with(
        None,
        vec![
            body(AUTHOR, "Radarr 502"),
            human(AUTHOR, "the first line of the conversation, quite long indeed"),
            human(HELPER, "the second line of the conversation, also long"),
        ],
        110,
    )
    .expect("one line fits");

    assert_eq!(
        conversation(&rendered),
        "Helper 1: the second line of the conversation, also long"
    );
}

fn confirmed_thread() -> Option<String> {
    thread(vec![
        body(AUTHOR, "Issue: Radarr 502, it worked yesterday"),
        human(HELPER, "Restarting nginx should fix it for you"),
        human(AUTHOR, "Restarted nginx and\n   that fixed the 502 completely"),
    ])
}

#[test]
fn the_user_confirming_in_the_conversation_counts() {
    assert!(user_said(
        &confirmed_thread().expect("the thread has usable messages"),
        "that fixed the 502 completely"
    ));
    assert!(user_said(
        &confirmed_thread().expect("the thread has usable messages"),
        "Restarted nginx and that fixed"
    ));
}

#[test]
fn a_helper_or_the_opening_post_is_not_the_user_confirming() {
    let rendered = confirmed_thread().expect("the thread has usable messages");

    assert!(!user_said(&rendered, "should fix it for you"));
    assert!(!user_said(&rendered, "it worked yesterday"));
    assert!(!user_said(&rendered, "never said this"));
    assert!(!user_said(&rendered, "   "));
}
