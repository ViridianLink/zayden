//! What the triage model is actually told about a new ticket.
//!
//! The follow-up questions are only as good as the prompt: a forum tag naming
//! the game version and a linked log both answer questions the model would
//! otherwise ask, so both have to reach it. These tests pin the presence of
//! that context, not its wording.

use ticket::faq::hit::{FaqHit, FaqSource};
use ticket::faq::linked::LinkedPage;
use ticket::faq::triage::{Opening, user_prompt};

fn hit() -> FaqHit {
    FaqHit {
        title: String::from("Port already in use"),
        description: String::from("Free the port before starting the server"),
        path: String::from("servers/ports"),
        source: FaqSource::Wiki,
    }
}

fn page() -> LinkedPage {
    LinkedPage {
        url: String::from("https://paste.ee/r/abc"),
        text: String::from("Caused by: java.net.BindException"),
    }
}

#[test]
fn the_title_reaches_the_model() {
    let prompt = user_prompt(
        Opening {
            title: "Server will not start",
            tags: &[],
            message: "it just dies",
            links: &[],
        },
        &[],
    );

    assert!(prompt.contains("Server will not start"), "{prompt}");
}

#[test]
fn the_tags_the_user_picked_reach_the_model() {
    let tags = [String::from("Palworld"), String::from("v0.3.11")];

    let prompt = user_prompt(
        Opening {
            title: "Crash on join",
            tags: &tags,
            message: "it crashes",
            links: &[],
        },
        &[],
    );

    assert!(prompt.contains("Palworld"), "{prompt}");
    assert!(prompt.contains("v0.3.11"), "{prompt}");
}

#[test]
fn a_ticket_with_no_tags_says_so_rather_than_leaving_a_blank() {
    let prompt = user_prompt(
        Opening {
            title: "Crash on join",
            tags: &[],
            message: "it crashes",
            links: &[],
        },
        &[],
    );

    assert!(prompt.contains("(none)"), "{prompt}");
}

#[test]
fn a_linked_page_arrives_as_text_not_just_a_url() {
    let links = [page()];

    let prompt = user_prompt(
        Opening {
            title: "Server will not start",
            tags: &[],
            message: "log is at https://paste.ee/r/abc",
            links: &links,
        },
        &[],
    );

    assert!(prompt.contains("https://paste.ee/r/abc"), "{prompt}");
    assert!(prompt.contains("Caused by: java.net.BindException"), "{prompt}");
}

#[test]
fn the_candidate_articles_still_ride_along() {
    let hits = [hit()];

    let prompt = user_prompt(
        Opening {
            title: "Server will not start",
            tags: &[],
            message: "it just dies",
            links: &[],
        },
        &hits,
    );

    assert!(prompt.contains("servers/ports"), "{prompt}");
    assert!(prompt.contains("Port already in use"), "{prompt}");
}

#[test]
fn the_users_own_words_are_never_dropped() {
    let prompt = user_prompt(
        Opening {
            title: "Server will not start",
            tags: &[String::from("Palworld")],
            message: "exit code 137 after about a minute",
            links: &[page()],
        },
        &[hit()],
    );

    assert!(prompt.contains("exit code 137 after about a minute"), "{prompt}");
}
