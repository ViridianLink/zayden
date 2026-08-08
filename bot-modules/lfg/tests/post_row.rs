//! `PostBuilder` ⇄ `PostRow` conversion and the two `TemplateInfo` impls.
//!
//! Audit finding lfg #2 (`design-docs/audits/lfg.md`). Every post that is
//! edited or copied makes the round trip `PostRow → PostBuilder → PostRow`
//! (`components/edit.rs`, `components/copy.rs`), and the embed the user sees is
//! rendered from whichever of the two types is at hand — a builder before the
//! post is persisted, a row afterwards. Both directions must therefore agree
//! field-for-field, or an edit silently drops data and the pre/post-save embeds
//! disagree.

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use lfg::PostBuilder;
use lfg::templates::TemplateInfo;
use serenity::all::{GenericChannelId, MessageId, ThreadId, UserId};
use zayden_core::as_i64;

const OWNER: u64 = 211_486_447_369_322_496;
const THREAD: u64 = 1_099_425_082_890_113_024;
const SCHEDULE_CHANNEL: u64 = 906_513_020_320_886_814;
const ALT_MESSAGE: u64 = 1_100_000_000_000_000_001;
const START: i64 = 1_800_000_000;

fn zoned(seconds: i64) -> Zoned {
    Timestamp::from_second(seconds)
        .unwrap_or(Timestamp::UNIX_EPOCH)
        .to_zoned(TimeZone::UTC)
}

fn builder() -> PostBuilder {
    PostBuilder::new(
        UserId::new(OWNER),
        "Vault of Glass",
        zoned(START),
        "Fresh run, mic required",
        6,
    )
    .id(ThreadId::new(THREAD))
    .schedule_channel(GenericChannelId::new(SCHEDULE_CHANNEL))
    .alt_message(MessageId::new(ALT_MESSAGE))
}

/// Assert two `TemplateInfo` views describe the same post.
fn assert_same_post(left: &impl TemplateInfo, right: &impl TemplateInfo) {
    assert_eq!(left.activity(), right.activity(), "activity");
    assert_eq!(left.timestamp(), right.timestamp(), "timestamp");
    assert_eq!(left.description(), right.description(), "description");
    assert_eq!(left.fireteam_size(), right.fireteam_size(), "fireteam_size");
    assert_eq!(
        left.fireteam().collect::<Vec<_>>(),
        right.fireteam().collect::<Vec<_>>(),
        "fireteam",
    );
    assert_eq!(
        left.alternatives().collect::<Vec<_>>(),
        right.alternatives().collect::<Vec<_>>(),
        "alternatives",
    );
    assert_eq!(
        left.schedule_channel(),
        right.schedule_channel(),
        "schedule_channel",
    );
    assert_eq!(left.alt_message(), right.alt_message(), "alt_message");
}

#[test]
fn builder_seeds_the_fireteam_with_the_owner() {
    // A fresh post is never empty: its creator occupies the first slot, which
    // is what makes `/lfg create` immediately render "Joined: 1/6".
    let row = builder().build();

    assert_eq!(row.fireteam, vec![as_i64(OWNER)]);
    assert_eq!(row.alternatives, Vec::<i64>::new());
}

#[test]
fn build_carries_every_field_onto_the_row() {
    let row = builder().build();

    assert_eq!(row.id, as_i64(THREAD));
    assert_eq!(row.owner_id, as_i64(OWNER));
    assert_eq!(row.activity, "Vault of Glass");
    assert_eq!(row.description, "Fresh run, mic required");
    assert_eq!(row.fireteam_size, 6);
    assert_eq!(row.start_time.to_jiff().as_second(), START);
    assert_eq!(row.alt_channel, Some(as_i64(SCHEDULE_CHANNEL)));
    assert_eq!(row.alt_message, Some(as_i64(ALT_MESSAGE)));
}

#[test]
fn optional_scheduling_fields_stay_absent_when_unset() {
    let row =
        PostBuilder::new(UserId::new(OWNER), "Crota's End", zoned(START), "", 6)
            .build();

    assert_eq!(row.alt_channel, None);
    assert_eq!(row.alt_message, None);
}

#[test]
fn row_ids_decode_back_to_discord_ids() {
    let row = builder().build();

    assert_eq!(row.thread(), ThreadId::new(THREAD));
    assert_eq!(row.owner(), UserId::new(OWNER));
    // A thread's starter message shares the thread's snowflake, which is why
    // `message()` reads the same column as `thread()`.
    assert_eq!(row.message(), MessageId::new(THREAD));
}

#[test]
fn row_to_builder_to_row_is_lossless() {
    let original = builder().build();
    let round_tripped = PostBuilder::from(original.clone()).build();

    assert_eq!(round_tripped.id, original.id);
    assert_eq!(round_tripped.owner_id, original.owner_id);
    assert_eq!(round_tripped.activity, original.activity);
    assert_eq!(round_tripped.description, original.description);
    assert_eq!(round_tripped.fireteam_size, original.fireteam_size);
    assert_eq!(round_tripped.fireteam, original.fireteam);
    assert_eq!(round_tripped.alternatives, original.alternatives);
    assert_eq!(round_tripped.alt_channel, original.alt_channel);
    assert_eq!(round_tripped.alt_message, original.alt_message);
    assert_eq!(round_tripped.start_time.to_jiff(), original.start_time.to_jiff(),);
}

#[test]
fn round_trip_preserves_a_full_roster() {
    // The edit/copy path rebuilds from a persisted row, so the roster must
    // survive the trip through the builder — which seeds only the owner when
    // constructed fresh.
    let mut row = builder().build();
    row.fireteam = vec![as_i64(OWNER), as_i64(OWNER + 1), as_i64(OWNER + 2)];
    row.alternatives = vec![as_i64(OWNER + 3)];

    let round_tripped = PostBuilder::from(row.clone()).build();

    assert_eq!(round_tripped.fireteam, row.fireteam);
    assert_eq!(round_tripped.alternatives, row.alternatives);
}

#[test]
fn round_trip_normalises_the_zone_but_keeps_the_instant() {
    // `From<PostRow>` rebuilds the start time as UTC. A post created in the
    // author's local zone therefore comes back UTC-zoned, but the *instant*
    // must be identical — that instant is what the `<t:…>` embed timestamp and
    // the reminder cron both read.
    let local = jiff::tz::db().get("America/New_York").unwrap_or(TimeZone::UTC);
    let start = zoned(START).with_time_zone(local);
    let row =
        PostBuilder::new(UserId::new(OWNER), "Last Wish", start.clone(), "", 6)
            .build();

    let rebuilt = PostBuilder::from(row);

    assert_eq!(rebuilt.timestamp(), start.timestamp());
    assert_eq!(rebuilt.timestamp().as_second(), START);
}

#[test]
fn builder_and_row_render_identically() {
    // Both types feed the same `embed()` renderer; drift between the impls
    // would show as a different embed before and after the post is saved.
    let post = builder();
    let row = builder().build();

    assert_same_post(&post, &row);
}

#[test]
fn rebuilt_builder_and_its_row_render_identically() {
    let row = builder().build();
    let rebuilt = PostBuilder::from(row.clone());

    assert_same_post(&rebuilt, &row);
}
