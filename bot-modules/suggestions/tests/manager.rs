//! Coverage for `SuggestionsGuildRow`'s snowflake accessors, moved into the
//! crate as part of the CC-1 concrete-`PgPool` migration. The nullable
//! `BIGINT` settings columns map to `ChannelId`s; a column-order or
//! None-handling slip would silently misroute (or drop) suggestions.
//!
//! (The promote/demote decision is pinned separately in `review_threshold.rs`.)

use serenity::all::ChannelId;
use suggestions::SuggestionsGuildRow;

#[test]
fn channel_accessors_map_configured_snowflakes() {
    let row = SuggestionsGuildRow {
        id: 1,
        suggestions_channel_id: Some(1_234_567_890),
        review_channel_id: Some(9_876_543_210),
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
    };
    assert!(row.channel_id().is_none());
    assert!(row.review_channel_id().is_none());
}
