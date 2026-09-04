//! Who a reminder pings, and what it says.
//!
//! The rule that matters: a thread going quiet must never ping a whole role
//! when there is a specific person who owes the reply. The role fallback exists
//! only for a ticket nobody has answered at all.

use ticket::idle::{Ball, Nudge, reminder};
use ticket::{RoleId, UserId};

const OP: UserId = UserId::new(1000);
const HELPER: UserId = UserId::new(2000);
const SUPPORT: RoleId = RoleId::new(100);
const SINCE: i64 = 1_700_000_000;

#[test]
fn the_poster_is_nudged_when_a_helper_spoke_last() {
    let r = reminder(Ball::Op, OP, Some(HELPER), &[SUPPORT]).unwrap();

    assert_eq!(r.kind, Nudge::Op);
    assert_eq!(r.users, vec![OP]);
    assert_eq!(r.roles, Vec::<RoleId>::new());
}

/// The regression this design exists to avoid: one stale thread must not ping
/// the entire support role when a named helper is already on it.
#[test]
fn a_known_helper_is_nudged_alone() {
    let r = reminder(Ball::Helper, OP, Some(HELPER), &[SUPPORT]).unwrap();

    assert_eq!(r.kind, Nudge::Helper);
    assert_eq!(r.users, vec![HELPER]);
    assert_eq!(r.roles, Vec::<RoleId>::new());
}

#[test]
fn an_unanswered_ticket_falls_back_to_the_support_roles() {
    let r = reminder(Ball::Helper, OP, None, &[SUPPORT]).unwrap();

    assert_eq!(r.kind, Nudge::Unanswered);
    assert_eq!(r.roles, vec![SUPPORT]);
    assert_eq!(r.users, Vec::<UserId>::new());
}

/// With no roles configured there is nobody to reach, and the guild owner is
/// deliberately not a fallback - a recurring background ping is not the same as
/// the one-off ping a ticket-open sends.
#[test]
fn an_unconfigured_guild_gets_no_reminder() {
    assert!(reminder(Ball::Helper, OP, None, &[]).is_none());
}

/// The poster is still reachable even with no roles set, because the reminder
/// is addressed to them by id.
#[test]
fn the_poster_is_reachable_without_support_roles() {
    assert!(reminder(Ball::Op, OP, None, &[]).is_some());
}

#[test]
fn only_the_posters_reminder_offers_buttons() {
    let op = reminder(Ball::Op, OP, Some(HELPER), &[SUPPORT]).unwrap();
    assert_eq!(op.components().len(), 1);

    for ball in [
        reminder(Ball::Helper, OP, Some(HELPER), &[SUPPORT]).unwrap(),
        reminder(Ball::Helper, OP, None, &[SUPPORT]).unwrap(),
    ] {
        assert!(ball.components().is_empty());
    }
}

#[test]
fn the_mention_line_leads_the_body() {
    let r = reminder(Ball::Op, OP, Some(HELPER), &[SUPPORT]).unwrap();
    let text = r.text(SINCE);

    assert!(text.starts_with("<@1000>\n"));
    assert!(text.contains("<t:1700000000:R>"));
}

#[test]
fn a_role_reminder_mentions_the_role_not_the_poster() {
    let r = reminder(Ball::Helper, OP, None, &[SUPPORT]).unwrap();

    assert_eq!(r.mentions(), "<@&100>");
}

/// Each of the three reads differently - a helper being chased must not be told
/// their own ticket has gone quiet.
#[test]
fn each_reminder_reads_differently() {
    let op = reminder(Ball::Op, OP, Some(HELPER), &[SUPPORT]).unwrap().text(SINCE);
    let helper =
        reminder(Ball::Helper, OP, Some(HELPER), &[SUPPORT]).unwrap().text(SINCE);
    let unanswered =
        reminder(Ball::Helper, OP, None, &[SUPPORT]).unwrap().text(SINCE);

    assert_ne!(op, helper);
    assert_ne!(helper, unanswered);
    assert_ne!(op, unanswered);
}
