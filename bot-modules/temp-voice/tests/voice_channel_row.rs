//! `VoiceChannelRow::new_persistent` constructor invariants: the caller becomes
//! the owner of an open, persistent channel with no trust/invite/password state.

use serenity::all::{ChannelId, UserId};
use temp_voice::VoiceChannelRow;
use temp_voice::voice_channel_manager::VoiceChannelMode;

#[test]
fn new_persistent_registers_caller_as_owner_of_an_open_channel() {
    let channel_id = ChannelId::new(123);
    let owner_id = UserId::new(456);

    let row = VoiceChannelRow::new_persistent(channel_id, owner_id);

    assert_eq!(row.channel_id(), channel_id);
    assert_eq!(row.owner_id(), owner_id);
    assert!(row.is_owner(owner_id));
    assert!(row.is_persistent());
    assert!(matches!(row.mode, VoiceChannelMode::Open));
    assert!(row.trusted_ids.is_empty());
    assert!(row.invites.is_empty());
    assert!(row.password.is_none());
}
