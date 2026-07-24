//! `has_manage_channels` gate used by claim/transfer to decide whether a
//! member may take over a temp voice channel.

use serenity::all::Permissions;
use temp_voice::commands::has_manage_channels;

#[test]
fn has_manage_channels_true_when_permission_granted() {
    assert!(has_manage_channels(Some(Permissions::MANAGE_CHANNELS)));
}

#[test]
fn has_manage_channels_true_with_other_permissions_present() {
    assert!(has_manage_channels(Some(
        Permissions::MANAGE_CHANNELS | Permissions::SEND_MESSAGES
    )));
}

#[test]
fn has_manage_channels_false_when_permission_absent() {
    assert!(!has_manage_channels(Some(Permissions::SEND_MESSAGES)));
}

#[test]
fn has_manage_channels_false_when_permissions_missing() {
    assert!(!has_manage_channels(None));
}
