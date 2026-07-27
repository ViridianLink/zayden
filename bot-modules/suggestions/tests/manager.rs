//! Coverage for `SuggestionsGuildRow`'s snowflake accessors, moved into the
//! crate as part of the CC-1 concrete-`PgPool` migration. The nullable
//! `BIGINT` settings columns map to `ChannelId`s; a column-order or
//! None-handling slip would silently misroute (or drop) suggestions.
//!
//! (The promote/demote decision is pinned separately in `review_threshold.rs`.)

use serenity::all::ChannelId;
use suggestions::{ReviewThresholds, SuggestionsGuildRow};

#[test]
fn channel_accessors_map_configured_snowflakes() {
    let row = SuggestionsGuildRow {
        id: 1,
        suggestions_channel_id: Some(1_234_567_890),
        review_channel_id: Some(9_876_543_210),
        promote_threshold: 20,
        demote_threshold: 15,
    };
    assert_eq!(row.channel_id().map(ChannelId::get), Some(1_234_567_890));
    assert_eq!(row.review_channel_id().map(ChannelId::get), Some(9_876_543_210));
}

#[test]
fn channel_accessors_pass_through_none() {
    let row = SuggestionsGuildRow {
        id: 1,
        suggestions_channel_id: None,
        review_channel_id: None,
        promote_threshold: 20,
        demote_threshold: 15,
    };
    assert!(row.channel_id().is_none());
    assert!(row.review_channel_id().is_none());
}

#[test]
fn row_carries_its_guild_thresholds_through() {
    // The columns must reach `review_action` unchanged — a swapped pair here
    // would silently restore the old global behaviour on a tuned guild.
    let row = SuggestionsGuildRow {
        id: 1,
        suggestions_channel_id: None,
        review_channel_id: None,
        promote_threshold: 4,
        demote_threshold: -2,
    };
    assert_eq!(row.thresholds(), ReviewThresholds::new(4, -2));
}
