//! The closing message only ever addresses the person who opened the ticket.

use ticket::UserId;
use ticket::idle::Notice;

const OP: UserId = UserId::new(1000);
const SINCE: i64 = 1_700_000_000;

#[test]
fn the_poster_is_mentioned_by_id() {
    assert!(Notice::new(OP, SINCE).text().starts_with("<@1000>"));
}

#[test]
fn the_text_dates_the_last_reply() {
    assert!(Notice::new(OP, SINCE).text().contains("<t:1700000000:R>"));
}

/// A closing ticket has nothing to ask of the support team, so it must not
/// reach for a role or a broadcast on its way out. Asserted against the payload
/// Discord actually receives rather than the builder's shape.
#[test]
fn nobody_but_the_poster_is_pinged() {
    let payload = serde_json::to_value(Notice::new(OP, SINCE).allowed_mentions())
        .expect("allowed mentions serialize");

    assert_eq!(payload["users"], serde_json::json!(["1000"]));
    assert_eq!(payload["roles"], serde_json::json!([]));
    assert_eq!(payload["parse"], serde_json::json!([]));
}
