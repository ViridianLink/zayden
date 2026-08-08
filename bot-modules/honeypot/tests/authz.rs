//! The `/honeypot` configuration gate (honeypot #7).
//!
//! Arming the honeypot points an auto-ban at a channel, so who may configure it
//! is the crate's second security-relevant decision after the exemption matrix
//! in `tests/policy.rs`. The gate is declared twice — once to Discord, as
//! `default_member_permissions(MANAGE_GUILD)` on the command, and once
//! server-side in `require_manage_guild` — and nothing verified the two still
//! agreed, or that the server-side half was called at all.
//!
//! `require_manage_guild` itself takes an `InvocationCtx`, which holds a
//! `&CommandInteraction` a test cannot build; the same fabrication problem that
//! ruled `verify` #1 untestable. So the *decision* was split out as
//! `is_privileged(Option<Permissions>)` and that is what these pin. The
//! *mapping* — that every subcommand is behind the gate — is now enforced by
//! construction instead of by test: the gate was hoisted into `Honeypot::run`
//! above the `match`, so there is one call site rather than three and a new
//! subcommand cannot bypass it.
//!
//! Everything here is offline and synchronous: `is_privileged` is a pure
//! predicate over a bitfield.

use honeypot::commands::is_privileged;
use honeypot::policy::is_staff;
use serenity::all::Permissions;

/// A DM, or any interaction payload that carries no member object.
///
/// This is the case that must fail closed, and the reason the predicate takes an
/// `Option` rather than a `Permissions`: reading the permission bits out of the
/// context is fallible, and "we could not tell" must never mean "allow".
#[test]
fn absent_permissions_are_not_privileged() {
    assert!(!is_privileged(None));
}

#[test]
fn empty_permissions_are_not_privileged() {
    assert!(!is_privileged(Some(Permissions::empty())));
}

#[test]
fn manage_guild_is_privileged() {
    assert!(is_privileged(Some(Permissions::MANAGE_GUILD)));
}

/// An ordinary member holding real permissions, none of them the gate's.
#[test]
fn unrelated_permissions_are_not_privileged() {
    assert!(!is_privileged(Some(Permissions::SEND_MESSAGES)));
    assert!(!is_privileged(Some(
        Permissions::SEND_MESSAGES
            | Permissions::MANAGE_MESSAGES
            | Permissions::KICK_MEMBERS
    )));
}

/// The gate must key off its own bit, not off the exact bitfield.
#[test]
fn manage_guild_alongside_other_permissions_is_privileged() {
    assert!(is_privileged(Some(
        Permissions::SEND_MESSAGES | Permissions::MANAGE_GUILD
    )));
    assert!(is_privileged(Some(Permissions::all())));
}

/// **Characterization test, not an endorsement — see honeypot #10.**
///
/// `Permissions::manage_guild()` is `contains(MANAGE_GUILD)`, a plain bitflag
/// test, so an `Administrator`-only bitfield does not satisfy this gate. The
/// crate's own `policy::is_staff` answers the same question as
/// `administrator() || manage_guild()`, and the dashboard gates guild
/// administration on that same pair — this predicate is the workspace's third
/// site and the only one testing a single bit.
///
/// Whether that is reachable depends on whether Discord expands `Administrator`
/// to a full bitfield in the interaction's computed `member.permissions`, which
/// cannot be settled from this repository. It is pinned here so the divergence
/// is visible rather than assumed. **If honeypot #10 widens the gate, this test
/// is expected to fail and should be updated, not deleted.**
#[test]
fn administrator_alone_does_not_satisfy_the_gate_but_does_satisfy_is_staff() {
    let admin = Permissions::ADMINISTRATOR;

    assert!(
        !is_privileged(Some(admin)),
        "honeypot #10: the gate ignores Administrator"
    );
    assert!(is_staff(admin), "honeypot #10: the exemption check does not");
}
