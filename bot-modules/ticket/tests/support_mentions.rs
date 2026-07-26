//! Who a new support ticket pings.
//!
//! This pins the user-visible half of `design-docs/audits/ticket.md` #2: the
//! dashboard used to write `support_settings.support_role_id`, which nothing
//! read, while the ticket flow pinged `guild_support_roles`, which nothing
//! wrote — so every ticket took the "no support role configured" branch and
//! woke the **server owner** instead of the configured support role. The fix
//! converged both sides on `guild_support_roles`; these tests pin the branch
//! that convergence feeds, so a future regression that strands the set again
//! shows up as the owner being pinged.
//!
//! The `guild_support_roles` statements themselves (`SupportRoles::{ids, add,
//! remove}`) need a live `PgPool`, for which this workspace has no test harness
//! yet — see [CC-6](../../../design-docs/audits/_cross-cutting.md).

use serenity::all::{Mention, RoleId, UserId};
use ticket::support_mentions;

const AUTHOR: UserId = UserId::new(1_000_000_000_000_000_001);
const OWNER: UserId = UserId::new(1_000_000_000_000_000_002);
const SUPPORT: RoleId = RoleId::new(1_300_000_000_000_000_003);
const ESCALATION: RoleId = RoleId::new(1_300_000_000_000_000_004);

/// `Mention` is not `PartialEq`, and the rendered form is what actually reaches
/// Discord: `send_support_message` concatenates exactly this.
fn rendered(mentions: &[Mention]) -> String {
    mentions.iter().map(ToString::to_string).collect()
}

const AUTHOR_PING: &str = "<@1000000000000000001>";
const OWNER_PING: &str = "<@1000000000000000002>";
const SUPPORT_PING: &str = "<@&1300000000000000003>";
const ESCALATION_PING: &str = "<@&1300000000000000004>";

#[test]
fn configured_roles_are_pinged_and_the_owner_is_left_alone() {
    let pings = rendered(&support_mentions(&[SUPPORT], AUTHOR, None));

    assert_eq!(pings, format!("{SUPPORT_PING}{AUTHOR_PING}"));
    assert!(
        !pings.contains(OWNER_PING),
        "a configured support role must not wake the guild owner"
    );
}

#[test]
fn every_configured_role_is_pinged() {
    let pings = rendered(&support_mentions(&[SUPPORT, ESCALATION], AUTHOR, None));

    assert_eq!(pings, format!("{SUPPORT_PING}{ESCALATION_PING}{AUTHOR_PING}"));
}

#[test]
fn owner_is_the_fallback_only_when_no_role_is_configured() {
    let pings = rendered(&support_mentions(&[], AUTHOR, Some(OWNER)));

    assert_eq!(pings, format!("{AUTHOR_PING}{OWNER_PING}"));
}

#[test]
fn unresolvable_owner_still_pings_the_author() {
    let pings = rendered(&support_mentions(&[], AUTHOR, None));

    assert_eq!(pings, AUTHOR_PING);
}
