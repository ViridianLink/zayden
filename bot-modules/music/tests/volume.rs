//! Guild volume as a persistent setting.
//!
//! Regression for the defect where `voice::start_playback` never applied
//! `GuildPlayer.volume` to the new handle: songbird defaults every fresh track
//! to 1.0, so `/music volume 30` was silently undone by the next track change.
//! Only the *scalar conversion* and the settings-refresh are unit-testable —
//! applying it to a live handle needs a running songbird driver — so those are
//! what this file pins.

use music::{GuildPlayer, volume_scalar};
use serenity::all::GenericChannelId;

#[test]
fn percentages_map_onto_songbirds_zero_to_one_scale() {
    assert!((volume_scalar(0) - 0.0).abs() < f32::EPSILON);
    assert!((volume_scalar(50) - 0.5).abs() < f32::EPSILON);
    assert!((volume_scalar(100) - 1.0).abs() < f32::EPSILON);
}

#[test]
fn volume_is_clamped_so_a_bad_value_cannot_blow_out_the_mix() {
    // The command validates 0-100, but `GuildPlayer.volume` is a plain u8 that
    // other paths write, so the conversion must not trust its input.
    assert!((volume_scalar(u8::MAX) - 1.0).abs() < f32::EPSILON);
    assert!(volume_scalar(u8::MAX) <= 1.0);
}

#[test]
fn a_new_player_starts_at_the_guilds_default_volume() {
    let player = GuildPlayer::new(GenericChannelId::new(1), 30);

    assert_eq!(player.volume, 30);
}

#[test]
fn the_scalar_applied_to_a_new_track_follows_the_players_volume() {
    // This is the value `start_playback` now feeds to `handle.set_volume`.
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);
    player.volume = 30;

    assert!((volume_scalar(player.volume) - 0.3).abs() < f32::EPSILON);
}
