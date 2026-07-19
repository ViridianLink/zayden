//! Regression coverage for the ownership-transition permission handling
//! (temp-voice DS-1).
//!
//! `claim`/`transfer` themselves need a live `Http` + Postgres pool, so the
//! DB/Discord round-trip is exercised end-to-end. Here we lock down the pure
//! decision that the fix hinges on: when ownership moves, the *previous* owner's
//! broad `owner_perms` overwrite must be revoked, not just the new owner granted.
//! Before the fix there was no revoke at all, so the former owner kept
//! `MANAGE_CHANNELS`/`MOVE_MEMBERS`/`MUTE_MEMBERS`/… via Discord's native UI.

use serenity::all::{PermissionOverwriteType, UserId};
use temp_voice::revoke_previous_owner;

#[test]
fn ownership_change_revokes_previous_owners_overwrite() {
    let previous = UserId::new(111);
    let new = UserId::new(222);

    // The old owner's member overwrite must be scheduled for deletion.
    assert_eq!(
        revoke_previous_owner(previous, new),
        Some(PermissionOverwriteType::Member(previous)),
    );
}

#[test]
fn self_transfer_does_not_revoke_just_granted_perms() {
    // Transferring/claiming to the same user must not delete the overwrite that
    // was just (re)granted to them.
    let user = UserId::new(333);

    assert_eq!(revoke_previous_owner(user, user), None);
}

#[test]
fn revoke_targets_the_previous_owner_not_the_new_one() {
    let previous = UserId::new(444);
    let new = UserId::new(555);

    match revoke_previous_owner(previous, new) {
        Some(PermissionOverwriteType::Member(id)) => assert_eq!(id, previous),
        other => {
            panic!("expected the previous owner's member overwrite, got {other:?}")
        },
    }
}
