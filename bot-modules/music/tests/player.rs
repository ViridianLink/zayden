use music::{GuildPlayer, LoopMode};
use serenity::all::GenericChannelId;

fn track(id: &str) -> music::ResolvedTrack {
    music::ResolvedTrack {
        title: id.to_string(),
        url: format!("https://youtu.be/{id}"),
        source_id: id.to_string(),
        source: music::TrackSource::YouTube,
        duration: None,
        is_live: false,
        thumbnail_url: None,
        requested_by: music::RequestedBy {
            user_id: serenity::all::UserId::new(1),
            display_name: "tester".to_string(),
        },
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
