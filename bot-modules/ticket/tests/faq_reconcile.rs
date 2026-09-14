//! Deciding what a new FAQ article does to the existing ones.
//!
//! The model proposes create, merge or discard; `reconcile::vet` holds it to
//! what code can check. Removing the candidate lookup fails
//! `a_target_outside_the_candidates_is_retried`, and removing the literal check
//! fails `a_discard_that_would_lose_a_command_is_retried`, which is the guard
//! against throwing away information the model did not recognise.

use jiff::Timestamp;
use ticket::faq::facts::literals;
use ticket::faq::reconcile::judge::{Action, Verdict};
use ticket::faq::reconcile::{Subject, Vetted, judge, merge, vet};
use ticket::{FaqArticle, NewArticle};

const THREAD: i64 = 1_547_382_547_653_206_016;

fn existing(id: i32, content: &str) -> FaqArticle {
    FaqArticle {
        id,
        guild_id: 1,
        title: format!("Existing {id}"),
        summary: String::from("One sentence of summary."),
        content: content.to_owned(),
        category: None,
        tags: Vec::new(),
        source_thread_id: Some(THREAD),
        generated: true,
        updated_at: Timestamp::UNIX_EPOCH.into(),
    }
}

fn verdict(action: Action, target_id: Option<i32>) -> Verdict {
    Verdict { action, target_id, reason: String::from("because") }
}

const fn subject(content: &str) -> Subject<'_> {
    Subject {
        article: NewArticle {
            title: "Radarr 502 behind nginx",
            summary: "The proxy cannot reach Radarr.",
            content,
            category: None,
            tags: &[],
        },
        dated: Timestamp::UNIX_EPOCH,
    }
}

#[test]
fn create_needs_no_target() {
    let candidates = [existing(7, "body")];

    assert!(matches!(
        vet(&verdict(Action::Create, None), &candidates, &literals("")),
        Vetted::Create
    ));
}

#[test]
fn a_target_outside_the_candidates_is_retried() {
    let candidates = [existing(7, "body")];

    for action in [Action::Merge, Action::Discard] {
        assert!(matches!(
            vet(&verdict(action, Some(99)), &candidates, &literals("")),
            Vetted::Retry(_)
        ));
    }
}

#[test]
fn a_merge_into_a_candidate_is_accepted() {
    let candidates = [existing(7, "body"), existing(8, "other")];

    assert!(matches!(
        vet(&verdict(Action::Merge, Some(8)), &candidates, &literals("")),
        Vetted::Merge(target) if target.id == 8
    ));
}

#[test]
fn a_discard_that_would_lose_a_command_is_retried() {
    let candidates = [existing(7, "Run `docker compose pull`.")];
    let draft = literals("Run `docker compose pull` then `docker compose up -d`.");

    let Vetted::Retry(feedback) =
        vet(&verdict(Action::Discard, Some(7)), &candidates, &draft)
    else {
        panic!("a discard losing a command must be retried");
    };

    assert!(feedback.contains("docker compose up -d"), "{feedback}");
}

#[test]
fn a_discard_fully_covered_by_its_target_is_accepted() {
    let candidates =
        [existing(7, "Run `docker compose pull` then `docker compose up -d`.")];
    let draft = literals("Just run `docker compose up -d`.");

    assert!(matches!(
        vet(&verdict(Action::Discard, Some(7)), &candidates, &draft),
        Vetted::Discard(target) if target.id == 7
    ));
}

#[test]
fn an_article_is_dated_by_its_source_ticket() {
    assert_eq!(
        existing(7, "body").dated().strftime("%Y-%m-%d").to_string(),
        "2026-09-09"
    );
}

#[test]
fn a_hand_written_article_is_dated_by_its_last_edit() {
    let mut article = existing(7, "body");
    article.source_thread_id = None;

    assert_eq!(article.dated(), Timestamp::UNIX_EPOCH);
}

#[test]
fn the_judge_sees_every_candidate_with_its_id_and_date() {
    let prompt = judge::user_prompt(
        subject("new body"),
        &[existing(7, "first body"), existing(8, "second body")],
        None,
    );

    for expected in
        ["new body", "id 7", "first body", "id 8", "second body", "2026-09-09"]
    {
        assert!(prompt.contains(expected), "{expected:?} missing from {prompt}");
    }
}

#[test]
fn the_judge_is_told_why_its_last_answer_was_rejected() {
    let prompt = judge::user_prompt(
        subject("body"),
        &[existing(7, "body")],
        Some("lost `x`"),
    );

    assert!(prompt.contains("lost `x`"), "{prompt}");
}

#[test]
fn the_writer_is_told_which_details_it_dropped() {
    let prompt =
        merge::user_prompt(subject("new body"), &existing(7, "old body"), &[
            String::from("docker compose up -d"),
        ]);

    for expected in ["new body", "old body", "`docker compose up -d`"] {
        assert!(prompt.contains(expected), "{expected:?} missing from {prompt}");
    }
}

#[test]
fn server_context_reaches_both_models_only_when_given() {
    for system_prompt in [judge::system_prompt, merge::system_prompt] {
        assert!(
            system_prompt(Some("A Palworld hosting server.")).contains("Palworld")
        );
        assert!(!system_prompt(None).contains("About this server"));
        assert!(!system_prompt(Some("  ")).contains("About this server"));
    }
}
