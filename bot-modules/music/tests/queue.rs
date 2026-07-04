use std::collections::HashSet;

use music::{MusicError, Queue, RequestedBy, ResolvedTrack, TrackSource};
use serenity::all::UserId;

fn track(source_id: &str, user_id: u64) -> ResolvedTrack {
    ResolvedTrack {
        title: source_id.to_string(),
        url: format!("https://youtu.be/{source_id}"),
        source_id: source_id.to_string(),
        source: TrackSource::YouTube,
        duration: None,
        is_live: false,
        thumbnail_url: None,
        requested_by: RequestedBy {
            user_id: UserId::new(user_id),
            display_name: "tester".to_string(),
        },
    }
}

#[test]
fn push_and_insert_top_order_correctly() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 1));
    q.insert_top(track("c", 1));

    assert_eq!(q.len(), 3);
    assert_eq!(q.get(0).map(|t| t.source_id.as_str()), Some("c"));
    assert_eq!(q.get(1).map(|t| t.source_id.as_str()), Some("a"));
    assert_eq!(q.get(2).map(|t| t.source_id.as_str()), Some("b"));
}

#[test]
fn remove_out_of_range_errors() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    assert!(matches!(q.remove(5), Err(MusicError::QueuePositionOutOfRange(5))));
    assert_eq!(q.len(), 1);
}

#[test]
fn remove_in_range_shrinks_queue() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 1));
    let removed = q.remove(0).expect("in range");
    assert_eq!(removed.source_id, "a");
    assert_eq!(q.len(), 1);
    assert_eq!(q.get(0).map(|t| t.source_id.as_str()), Some("b"));
}

#[test]
fn skip_to_drops_preceding_tracks() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 1));
    q.push(track("c", 1));

    let target = q.skip_to(2).expect("in range");
    assert_eq!(target.source_id, "c");
    assert!(q.is_empty());
}

#[test]
fn skip_to_out_of_range_errors_without_mutating() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    assert!(q.skip_to(3).is_err());
    assert_eq!(q.len(), 1);
}

#[test]
fn move_song_reorders() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 1));
    q.push(track("c", 1));

    q.move_song(0, 2).expect("in range");
    assert_eq!(q.get(0).map(|t| t.source_id.as_str()), Some("b"));
    assert_eq!(q.get(1).map(|t| t.source_id.as_str()), Some("c"));
    assert_eq!(q.get(2).map(|t| t.source_id.as_str()), Some("a"));
}

#[test]
fn move_song_out_of_range_errors() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    assert!(q.move_song(0, 5).is_err());
    assert!(q.move_song(5, 0).is_err());
}

#[test]
fn dedupe_keeps_first_occurrence() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 1));
    q.push(track("a", 1));

    let removed = q.dedupe();
    assert_eq!(removed, 1);
    assert_eq!(q.len(), 2);
    assert_eq!(q.get(0).map(|t| t.source_id.as_str()), Some("a"));
    assert_eq!(q.get(1).map(|t| t.source_id.as_str()), Some("b"));
}

#[test]
fn shuffle_preserves_all_tracks() {
    let mut q = Queue::new();
    for i in 0..10 {
        q.push(track(&i.to_string(), 1));
    }
    q.shuffle();
    assert_eq!(q.len(), 10);

    let mut ids: Vec<_> = q.iter().map(|t| t.source_id.clone()).collect();
    ids.sort();
    assert_eq!(ids, (0..10).map(|i| i.to_string()).collect::<Vec<_>>());
}

#[test]
fn cleanup_removes_tracks_from_absent_requesters() {
    let mut q = Queue::new();
    q.push(track("a", 1));
    q.push(track("b", 2));
    q.push(track("c", 1));

    let present: HashSet<UserId> = std::iter::once(UserId::new(1)).collect();
    let removed = q.cleanup(&present);

    assert_eq!(removed, 1);
    assert_eq!(q.len(), 2);
    assert!(q.iter().all(|t| t.requested_by.user_id == UserId::new(1)));
}
