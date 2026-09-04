//! Which channels the ticket lifecycle commands will act on.
//!
//! `/ticket close`, `open` and `solved` all rename the channel they are run
//! in. They used to reach that rename through `expect_thread()`, which only
//! *relabels* an id rather than checking it, so running one in an ordinary text
//! channel renamed **that channel** to "[Closed] - general". The guard they now
//! share has to reject anything that is not a support thread before the rename.
//!
//! A ticket is a thread either way, so both thread shapes have to pass: a post
//! in a forum, and a thread under a text channel.

use serenity::all::{ChannelId, GenericInteractionChannel};
use ticket::support_thread;

const SUPPORT: ChannelId = ChannelId::new(1_307_003_078_536_462_407);
const ELSEWHERE: ChannelId = ChannelId::new(1_307_003_078_536_462_408);
const THREAD: u64 = 1_545_215_001_378_431_006;

/// `kind` 11 is a public thread (what a forum post is), 12 a private thread
/// (what the modal and message flows open under a text channel).
fn thread(kind: u8, parent: ChannelId) -> Option<GenericInteractionChannel> {
    let value = serde_json::json!({
        "type": kind,
        "id": THREAD.to_string(),
        "parent_id": parent.get().to_string(),
        "guild_id": "1120465621554040942",
        "name": "1 - reporter - my mod menu wont load",
        "last_message_id": null,
        "thread_metadata": {
            "archived": false,
            "auto_archive_duration": 10080,
            "archive_timestamp": null,
            "locked": false,
            "create_timestamp": null,
        },
    });

    serde_json::from_value(value).ok().map(GenericInteractionChannel::Thread)
}

/// `kind` 0 is a text channel, 15 a forum.
fn channel(kind: u8) -> Option<GenericInteractionChannel> {
    let value = serde_json::json!({
        "type": kind,
        "id": SUPPORT.get().to_string(),
        "guild_id": "1120465621554040942",
        "name": "support",
        "last_message_id": null,
        "position": 0,
        "topic": null,
        "nsfw": false,
        "parent_id": null,
    });

    serde_json::from_value(value).ok().map(GenericInteractionChannel::Channel)
}

#[test]
fn a_forum_post_in_the_support_forum_is_a_ticket() {
    let channel = thread(11, SUPPORT).expect("thread fixture deserializes");

    let thread = support_thread(&channel, SUPPORT).expect("forum post accepted");

    assert_eq!(thread.parent_id, SUPPORT);
}

#[test]
fn a_private_thread_under_the_support_channel_is_a_ticket() {
    let channel = thread(12, SUPPORT).expect("thread fixture deserializes");

    let thread = support_thread(&channel, SUPPORT).expect("thread accepted");

    assert_eq!(thread.parent_id, SUPPORT);
}

/// The regression: a plain text channel is not a ticket, and must not be
/// renamed as though it were one.
#[test]
fn a_text_channel_is_not_a_ticket() {
    let channel = channel(0).expect("channel fixture deserializes");

    assert!(support_thread(&channel, SUPPORT).is_err());
}

/// Nor is the support forum itself — only the posts inside it.
#[test]
fn the_forum_channel_itself_is_not_a_ticket() {
    let channel = channel(15).expect("channel fixture deserializes");

    assert!(support_thread(&channel, SUPPORT).is_err());
}

#[test]
fn a_thread_under_some_other_channel_is_not_a_ticket() {
    let post = thread(11, ELSEWHERE).expect("thread fixture deserializes");
    let private = thread(12, ELSEWHERE).expect("thread fixture deserializes");

    assert!(support_thread(&post, SUPPORT).is_err());
    assert!(support_thread(&private, SUPPORT).is_err());
}
