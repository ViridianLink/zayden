//! Who may mark a ticket solved, from the idle reminder's buttons or from
//! `/ticket solved`.
//!
//! The rest of `/ticket` is gated on Manage Messages, which would lock the
//! poster out of their own ticket and lock out helpers who only hold a support
//! role. Solving widens that to the poster and to anyone holding a support
//! role, and to nobody else.

use ticket::RoleId;
use ticket::idle::may_act;

const OP: u64 = 1000;
const SUPPORT: RoleId = RoleId::new(100);
const UNRELATED: RoleId = RoleId::new(999);

const fn user(id: u64) -> ticket::UserId {
    ticket::UserId::new(id)
}

#[test]
fn the_poster_may_act_on_their_own_ticket() {
    assert!(may_act(user(OP), Some(user(OP)), &[], &[SUPPORT], false));
}

#[test]
fn a_support_role_holder_may_act() {
    assert!(may_act(user(7), Some(user(OP)), &[SUPPORT], &[SUPPORT], false));
}

/// Parity with `require_manage`, so a moderator without the support role is not
/// locked out of a ticket they are already allowed to close.
#[test]
fn manage_messages_may_act() {
    assert!(may_act(user(7), Some(user(OP)), &[UNRELATED], &[SUPPORT], true));
}

#[test]
fn a_bystander_may_not_act() {
    assert!(!may_act(user(7), Some(user(OP)), &[UNRELATED], &[SUPPORT], false));
}

/// With no support roles configured, holding a role cannot make anyone a
/// helper, so only the poster and moderators get through.
#[test]
fn an_unconfigured_guild_grants_nobody_a_role_path() {
    assert!(!may_act(user(7), Some(user(OP)), &[SUPPORT], &[], false));
    assert!(may_act(user(OP), Some(user(OP)), &[SUPPORT], &[], false));
}

/// A ticket opened before activity tracking has no recorded poster, so nobody
/// gets in on the poster path.
#[test]
fn an_unknown_poster_grants_nobody_the_poster_path() {
    assert!(!may_act(user(OP), None, &[UNRELATED], &[SUPPORT], false));
    assert!(may_act(user(OP), None, &[SUPPORT], &[SUPPORT], false));
}
