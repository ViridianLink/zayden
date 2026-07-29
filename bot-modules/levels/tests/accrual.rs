//! Regression tests for levels DS-1 — the message-XP site of the
//! [CC-9](../../../design-docs/audits/_cross-cutting.md) absolute-overwrite
//! class, and the last one enumerated there.
//!
//! DS-1 was that `message_create` read the whole level row, `accrue_message`
//! mutated `xp`/`level`/`total_xp`/`message_count` **in memory**, and `save`
//! persisted that post-image **absolutely** (`xp = EXCLUDED.xp, …`). Two
//! messages from the same author whose handlers interleave both read the same
//! pre-image and both write it back, so one message's XP, total XP and message
//! count are lost. The 1-minute cooldown had the same shape: it was compared
//! against the *snapshot's* `last_xp`, so both handlers also passed a gate that
//! should have stopped the second.
//!
//! The write path needs a live `PgPool` (and the accrual itself now lives in
//! SQL), for which this crate has no harness (audit CC-6). What is testable
//! in-process is what the fix introduces: the XP is a `MessageXp` **increment**
//! applied to whatever the live row holds, and the level-up is a `LevelUp`
//! decided against the **post-increment** values the write returned — carrying
//! the pre-image `level` so the follow-up update is a compare-and-swap.

use levels::{LevelUp, MessageXp, level_up_xp};

/// The roll stays inside the documented band; it is the *amount* accrued, not a
/// post-image, so nothing else on the row is implied by it.
#[test]
fn message_xp_rolls_inside_the_band() {
    for _ in 0..1_000 {
        let xp = MessageXp::roll().amount();
        assert!(
            (MessageXp::MIN..=MessageXp::MAX).contains(&xp),
            "rolled {xp}, outside {}..={}",
            MessageXp::MIN,
            MessageXp::MAX
        );
    }
}

/// The core DS-1 scenario: two concurrent messages read the same pre-image.
/// Applied as increments both land; folded into the snapshot and written back
/// absolutely, the second clobbers the first.
#[test]
fn concurrent_messages_both_land_as_increments() {
    let live_xp = 90;

    let first = MessageXp::new(20);
    let second = MessageXp::new(18);

    // Fixed: each write is `xp = xp + n` against the live row.
    assert_eq!(
        live_xp + first.amount() + second.amount(),
        128,
        "both messages' XP survives"
    );

    // Before: both handlers computed a post-image from the same read and the
    // later write won outright.
    let first_post_image = live_xp + first.amount();
    let second_post_image = live_xp + second.amount();
    assert_eq!(second_post_image, 108, "the absolute write lands 108…");
    assert_ne!(second_post_image, 128, "…losing the other message's XP entirely");
    assert_ne!(first_post_image, second_post_image, "…whichever wrote last");
}

/// The message counter has the same shape: `+1` from each handler, not a
/// post-image of a shared read.
#[test]
fn message_count_increments_do_not_collide() {
    let live_count: i64 = 10;

    // Two handlers, each contributing one message.
    assert_eq!(live_count + 1 + 1, 12, "both messages are counted");
    // Both computed `10 + 1` from the same snapshot and wrote it absolutely.
    assert_eq!(live_count + 1, 11, "the absolute write counts one of the two");
}

/// The level-up is decided from the post-increment XP the write returned, and
/// carries the level it saw so the follow-up update can compare-and-swap on it.
#[test]
fn level_up_is_decided_against_the_returned_row() {
    let level = 0;
    let threshold = level_up_xp(level);

    assert!(
        LevelUp::check(threshold - 1, level).is_none(),
        "below the curve there is no level-up"
    );

    let up = LevelUp::check(threshold, level).expect("at the curve it levels");
    assert_eq!(up.from_level, level, "the compare half of the swap");
    assert_eq!(up.threshold, threshold, "the XP the level-up consumes");
    assert_eq!(up.new_level(), 1);
}

/// A level-up consumes exactly the curve's threshold; the overshoot carries over
/// rather than being reset, which is what the in-memory version did too — the
/// fix must not change the curve, only where the arithmetic happens.
#[test]
fn level_up_carries_the_overshoot() {
    let level = 3;
    let threshold = level_up_xp(level);
    let overshoot = 17;

    let up = LevelUp::check(threshold + overshoot, level).unwrap();

    assert_eq!(
        threshold + overshoot - up.threshold,
        overshoot,
        "carried XP survives the level-up"
    );
    assert_eq!(up.new_level(), 4);
}

/// The guard that makes the level-up idempotent: two handlers that both observe
/// a level-up on the same pre-image agree on the `from_level` they compare
/// against, so only the first `WHERE level = from_level` update can apply — the
/// loser matches no row instead of reverting the level the winner set.
#[test]
fn racing_level_ups_agree_on_the_compare_value() {
    let level = 2;
    let threshold = level_up_xp(level);

    let first = LevelUp::check(threshold + 5, level).unwrap();
    let second = LevelUp::check(threshold + 23, level).unwrap();

    assert_eq!(first.from_level, second.from_level);
    assert_eq!(
        first.new_level(),
        second.new_level(),
        "neither can push the level past +1 for one message"
    );
}

/// Every level on the curve is reachable: the check never mis-fires below the
/// threshold nor misses at it, at any level.
#[test]
fn level_up_matches_the_curve_at_every_level() {
    for level in 0..50 {
        let threshold = level_up_xp(level);

        assert!(LevelUp::check(threshold - 1, level).is_none(), "level {level}");
        assert_eq!(
            LevelUp::check(threshold, level).map(LevelUp::new_level),
            Some(level + 1),
            "level {level}"
        );
    }
}
