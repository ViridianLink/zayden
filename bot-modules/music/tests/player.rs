use music::{AnnounceConfig, GuildPlayer, LoopMode, MusicSettingsRow};
use serenity::all::{GenericChannelId, UserId};
use zayden_app::config::SettingsRow;

fn track(id: &str) -> music::ResolvedTrack {
    music::ResolvedTrack {
        title: id.to_string(),
        url: format!("https://youtu.be/{id}"),
        source_id: id.to_string(),
        source: music::TrackSource::YouTube,
        duration: None,
        is_live: false,
        thumbnail_url: None,
        requested_by: UserId::new(1),
    }
}

const fn is_stale(player: &GuildPlayer, captured_generation: u64) -> bool {
    player.generation != captured_generation
}

#[test]
fn concurrent_advance_makes_a_captured_generation_stale() {
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);
    let captured_generation = player.generation;

    // Simulate a concurrent forceskip/playnow, which also calls `advance`.
    let _ = player.advance();

    assert!(is_stale(&player, captured_generation));
}

#[test]
fn untouched_generation_is_not_stale() {
    let player = GuildPlayer::new(GenericChannelId::new(1), 100);
    let captured_generation = player.generation;

    assert!(!is_stale(&player, captured_generation));
}

#[test]
fn try_begin_start_reserves_the_idle_transition_exactly_once() {
    // DS-2 regression: two concurrent first-`/play` interactions on an idle
    // player both observed `current.is_none() == true` and both started a track
    // (overlapping audio + orphaned handle + double queue-advance). The atomic
    // reservation must let only the first caller start.
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);

    assert!(player.try_begin_start(), "first idle caller should start playback");
    assert!(
        !player.try_begin_start(),
        "second concurrent caller must enqueue only while a start is reserved"
    );
}

#[test]
fn finish_start_releases_the_reservation_for_a_later_idle_start() {
    // A start that fails (e.g. a resolve/stream error) must release the
    // reservation so a subsequent `/play` on the still-idle player can start.
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);

    assert!(player.try_begin_start());
    player.finish_start();
    assert!(
        player.try_begin_start(),
        "reservation should be reusable after finish_start"
    );
}

// DS-3 regression: `announce_now_playing` had no consumer at all, so the
// toggle was a no-op at every value. `announce_target` is the decision the
// `TrackEndNotifier` now makes before posting an unprompted announcement.
#[test]
fn announce_target_defaults_to_the_session_text_channel() {
    let command_channel = GenericChannelId::new(10);
    let player = GuildPlayer::new(command_channel, 100);

    assert_eq!(player.announce_target(), Some(command_channel));
}

#[test]
fn announce_target_prefers_the_configured_announce_channel() {
    let command_channel = GenericChannelId::new(10);
    let announce_channel = GenericChannelId::new(20);
    let mut player = GuildPlayer::new(command_channel, 100);

    player.set_announce(AnnounceConfig {
        enabled: true,
        channel: Some(announce_channel),
    });

    assert_eq!(player.announce_target(), Some(announce_channel));
}

#[test]
fn announce_target_is_none_when_announcements_are_disabled() {
    let mut player = GuildPlayer::new(GenericChannelId::new(10), 100);

    player.set_announce(AnnounceConfig {
        enabled: false,
        channel: Some(GenericChannelId::new(20)),
    });

    assert_eq!(
        player.announce_target(),
        None,
        "a disabled toggle must silence announcements even with a channel set"
    );
}

#[test]
fn announce_config_mirrors_the_guild_settings_row() {
    let mut row = MusicSettingsRow::empty(1);
    assert_eq!(AnnounceConfig::from(&row), AnnounceConfig {
        enabled: true,
        channel: None,
    });

    row.announce_now_playing = false;
    row.announce_channel_id = Some(20);
    assert_eq!(AnnounceConfig::from(&row), AnnounceConfig {
        enabled: false,
        channel: Some(GenericChannelId::new(20)),
    });
}

#[test]
fn a_new_player_is_not_silenced() {
    let channel = GenericChannelId::new(10);
    let player = GuildPlayer::new(channel, 100);

    assert!(!player.silenced);
    assert_eq!(player.announce_target(), Some(channel));
}

#[test]
fn silencing_the_session_suppresses_announcements() {
    let mut player = GuildPlayer::new(GenericChannelId::new(10), 100);
    player.set_announce(AnnounceConfig {
        enabled: true,
        channel: Some(GenericChannelId::new(20)),
    });

    player.silenced = true;

    assert_eq!(
        player.announce_target(),
        None,
        "a silenced session must stay quiet even with announcements enabled \
         and a dedicated channel configured"
    );
}

// `voice::ensure_session` re-reads the guild row and calls `set_announce` on
// every `/music play`, so session silence deliberately lives outside
// `AnnounceConfig` — otherwise queueing a track would un-silence the session.
#[test]
fn refreshing_the_guild_config_does_not_clear_session_silence() {
    let mut player = GuildPlayer::new(GenericChannelId::new(10), 100);
    player.silenced = true;

    player.set_announce(AnnounceConfig { enabled: true, channel: None });

    assert!(player.silenced);
    assert_eq!(player.announce_target(), None);
}

#[test]
fn clearing_silence_cannot_override_a_disabled_guild_setting() {
    let mut player = GuildPlayer::new(GenericChannelId::new(10), 100);
    player.set_announce(AnnounceConfig { enabled: false, channel: None });

    player.silenced = false;

    assert_eq!(
        player.announce_target(),
        None,
        "session silence only adds quiet; it never re-enables announcements \
         the guild has turned off"
    );
}

#[test]
fn advance_queue_loop_off_pops_next_and_drops_finished() {
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);
    player.queue.push(track("b"));

    let next = player.advance_queue().expect("queued track");
    assert_eq!(next.source_id, "b");
    assert!(player.queue.is_empty());
}

#[test]
fn advance_queue_loop_track_replays_finished_without_touching_queue() {
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);
    player.loop_mode = LoopMode::Track;
    player.queue.push(track("b"));

    let next = player.advance_queue();
    assert!(next.is_none());
    assert_eq!(player.queue.len(), 1);
}

#[test]
fn advance_queue_loop_queue_cycles_finished_to_the_back() {
    let mut player = GuildPlayer::new(GenericChannelId::new(1), 100);
    player.loop_mode = LoopMode::Queue;
    player.queue.push(track("b"));

    let next = player.advance_queue().expect("next in queue");
    assert_eq!(next.source_id, "b");
    assert!(player.queue.is_empty());
}
