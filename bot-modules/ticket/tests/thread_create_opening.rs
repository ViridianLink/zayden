//! Recovering the reporter and their words from a thread's opening message.
//!
//! The FAQ triage hangs off `THREAD_CREATE`, which carries the thread but
//! neither the reporter nor what they asked, and where those live depends on
//! how the ticket was opened:
//!
//! - The bot's own flows put the reporter's words in an `Issue` embed and the
//!   reporter in the ping line, because the thread's `owner_id` is the bot. `author`
//!   is then the exact inverse of [`support_mentions`], so these tests feed it that
//!   function's rendered output — a change to either side surfaces here rather than
//!   as tickets triaged at the wrong user.
//! - A support channel run as a forum reaches neither flow. The post *is* the
//!   thread, so the opening message is the reporter's own plain text.

use serenity::all::{Mention, RoleId, UserId};
use ticket::support_mentions;
use ticket::thread_create::{author, opening};

const AUTHOR: UserId = UserId::new(1_000_000_000_000_000_001);
const OWNER: UserId = UserId::new(1_000_000_000_000_000_002);
const SUPPORT: RoleId = RoleId::new(1_300_000_000_000_000_003);
const ESCALATION: RoleId = RoleId::new(1_300_000_000_000_000_004);

/// What `send_support_message` actually sends: the mentions concatenated.
fn rendered(mentions: &[Mention]) -> String {
    mentions.iter().map(ToString::to_string).collect()
}

#[test]
fn reads_the_author_back_out_of_an_owner_ping() {
    let content = rendered(&support_mentions(&[], AUTHOR, Some(OWNER)));

    assert_eq!(author(&content), Some(AUTHOR));
}

#[test]
fn reads_the_author_back_when_only_the_author_is_pinged() {
    let content = rendered(&support_mentions(&[], AUTHOR, None));

    assert_eq!(author(&content), Some(AUTHOR));
}

/// The reporter trails the role pings, so a parser that took the first mention
/// of any kind would return a role id cast to a user.
#[test]
fn role_pings_do_not_shadow_the_author() {
    let content = rendered(&support_mentions(&[SUPPORT, ESCALATION], AUTHOR, None));

    assert!(content.starts_with("<@&"), "expected roles first: {content}");
    assert_eq!(author(&content), Some(AUTHOR));
}

#[test]
fn legacy_nickname_mentions_still_resolve() {
    assert_eq!(author("<@!1000000000000000001>"), Some(AUTHOR));
}

#[test]
fn content_without_a_user_mention_yields_nothing() {
    assert_eq!(author(""), None);
    assert_eq!(author("<@&1300000000000000003>"), None);
    assert_eq!(author("no mentions here"), None);
}

/// `UserId::new` panics on `u64::MAX`, so the parser must reject it rather than
/// take down the triage task.
#[test]
fn out_of_range_ids_are_rejected() {
    assert_eq!(author(&format!("<@{}>", u64::MAX)), None);
    assert_eq!(author("<@not_a_number>"), None);
}

// --- Which message in the thread is the ticket -------------------------------

const REPORTER: UserId = UserId::new(1_000_000_000_000_000_005);

/// A support channel run as a forum never reaches either bot ticket flow: the
/// post *is* the thread, so the opening message is the reporter's own plain
/// text with no issue embed and no ping line to read the author out of. This
/// is the case that produced "opened without a readable message" in the logs.
#[test]
fn a_forum_post_is_its_own_ticket() {
    assert_eq!(
        opening(false, REPORTER, "my mod menu wont load", None),
        Some((REPORTER, String::from("my mod menu wont load"))),
    );
}

#[test]
fn a_bot_ticket_still_prefers_the_issue_embed() {
    let content = rendered(&support_mentions(&[SUPPORT], AUTHOR, None));

    assert_eq!(
        opening(true, OWNER, &content, Some("printer is on fire")),
        Some((AUTHOR, String::from("printer is on fire"))),
    );
}

/// The bot's own ping line is non-empty text, so a fallback that ignored who
/// wrote the message would file the mention string as the ticket body.
#[test]
fn the_bots_ping_line_is_never_mistaken_for_a_ticket() {
    let content = rendered(&support_mentions(&[SUPPORT], AUTHOR, None));

    assert_eq!(opening(true, OWNER, &content, None), None);
}

#[test]
fn a_post_with_no_words_is_not_triaged() {
    assert_eq!(opening(false, REPORTER, "   ", None), None);
    assert_eq!(opening(false, REPORTER, "", None), None);
}

#[test]
fn the_reporters_text_is_trimmed() {
    assert_eq!(
        opening(false, REPORTER, "  spaced out  ", None),
        Some((REPORTER, String::from("spaced out"))),
    );
}
