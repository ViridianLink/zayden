//! Recovering the reporter from a support thread's opening message.
//!
//! The FAQ triage now hangs off `THREAD_CREATE`, which carries the thread but
//! not the reporter: the thread's `owner_id` is the bot, because the bot is
//! what called `create_thread`. The only record of who opened the ticket is
//! the mention string `send_support_message` writes as the opening message's
//! content, so `author` parses it back.
//!
//! That makes `author` the exact inverse of [`support_mentions`], and the two
//! are only correct together — these tests feed it the rendered output of that
//! function so a change to either side shows up here rather than as tickets
//! silently triaged at the wrong user.

use serenity::all::{Mention, RoleId, UserId};
use ticket::support_mentions;
use ticket::thread_create::author;

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
