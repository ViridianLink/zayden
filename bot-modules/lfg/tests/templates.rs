//! Post embed rendering and the button ids the post carries.
//!
//! Audit finding lfg #2 (`design-docs/audits/lfg.md`). Every LFG post is a
//! `DefaultTemplate` embed plus two action rows, and the button `custom_id`s
//! emitted here are the exact strings `bot/src/bindings/lfg/mod.rs` routes on
//! (`IdMatch::Exact`). A rename on this side compiles cleanly and silently
//! deadens the button, so the emitted set is pinned.

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use lfg::PostRow;
use lfg::templates::{DefaultTemplate, Template};
use lfg::utils::Announcement;
use serde_json::Value;
use serenity::all::{CreateActionRow, CreateEmbed, ThreadId, UserId};
use zayden_core::as_i64;

const OWNER: u64 = 211_486_447_369_322_496;
const THREAD: u64 = 1_099_425_082_890_113_024;

/// Ids routed by `bot/src/bindings/lfg/mod.rs` from the post's main row.
const MAIN_ROW_IDS: [&str; 4] =
    ["lfg_alternative", "lfg_join", "lfg_leave", "lfg_settings"];

/// Ids routed by `bot/src/bindings/lfg/mod.rs` from the post's settings row.
const SETTINGS_ROW_IDS: [&str; 4] =
    ["lfg_copy", "lfg_delete", "lfg_edit", "lfg_kick"];

fn post(fireteam: &[u64], alternatives: &[u64], description: &str) -> PostRow {
    PostRow {
        id: as_i64(THREAD),
        owner_id: as_i64(OWNER),
        activity: "Vault of Glass".to_string(),
        start_time: Timestamp::UNIX_EPOCH.to_sqlx(),
        description: description.to_string(),
        fireteam_size: 6,
        fireteam: fireteam.iter().copied().map(as_i64).collect(),
        alternatives: alternatives.iter().copied().map(as_i64).collect(),
        alt_channel: None,
        alt_message: None,
    }
}

fn members(count: u64) -> Vec<u64> {
    (0..count).map(|i| OWNER + i).collect()
}

fn render(embed: &CreateEmbed<'_>) -> String {
    serde_json::to_string(embed).unwrap_or_default()
}

fn fields(embed: &CreateEmbed<'_>) -> Vec<(String, String)> {
    let json = serde_json::to_value(embed).unwrap_or(Value::Null);

    json.get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .map(|field| (text(field, "name"), text(field, "value")))
                .collect()
        })
        .unwrap_or_default()
}

fn text(field: &Value, key: &str) -> String {
    field.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// Every `custom_id` anywhere in a serialized action row, sorted.
fn custom_ids(row: &CreateActionRow<'_>) -> Vec<String> {
    let json = serde_json::to_value(row).unwrap_or(Value::Null);
    let mut ids = Vec::new();
    collect_custom_ids(&json, &mut ids);
    ids.sort();
    ids
}

fn collect_custom_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "custom_id"
                    && let Some(id) = child.as_str()
                {
                    out.push(id.to_string());
                }
                collect_custom_ids(child, out);
            }
        },
        Value::Array(items) => {
            for item in items {
                collect_custom_ids(item, out);
            }
        },
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {},
    }
}

#[test]
fn main_row_emits_exactly_the_routed_ids() {
    assert_eq!(custom_ids(&DefaultTemplate::main_row()), MAIN_ROW_IDS);
}

#[test]
fn settings_row_emits_exactly_the_routed_ids() {
    assert_eq!(custom_ids(&DefaultTemplate::settings_row()), SETTINGS_ROW_IDS);
}

#[test]
fn every_button_id_is_lfg_namespaced_and_unique() {
    let mut all = [MAIN_ROW_IDS, SETTINGS_ROW_IDS].concat();

    for id in &all {
        assert!(id.starts_with("lfg_"), "custom id `{id}` is not namespaced");
    }

    let before = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), before, "duplicate custom id across the two rows");
}

#[test]
fn embed_shows_the_joined_count_against_capacity() {
    let json =
        render(&DefaultTemplate::thread_embed(&post(&members(3), &[], ""), "Kilo"));

    assert!(json.contains("Joined: 3/6"), "missing joined counter: {json}");
}

#[test]
fn embed_lists_every_fireteam_member() {
    let json =
        render(&DefaultTemplate::thread_embed(&post(&members(3), &[], ""), "Kilo"));

    for id in members(3) {
        assert!(json.contains(&format!("<@{id}>")), "member {id} not rendered");
    }
}

#[test]
fn embed_credits_the_owner_in_the_footer() {
    let json =
        render(&DefaultTemplate::thread_embed(&post(&members(1), &[], ""), "Kilo"));

    assert!(json.contains("Posted by Kilo"), "missing footer: {json}");
}

#[test]
fn alternatives_field_appears_only_when_someone_is_an_alternate() {
    let without =
        render(&DefaultTemplate::thread_embed(&post(&members(2), &[], ""), "Kilo"));
    assert!(!without.contains("Alternatives"));

    let with = render(&DefaultTemplate::thread_embed(
        &post(&members(2), &[OWNER + 9], ""),
        "Kilo",
    ));
    assert!(with.contains("Alternatives"));
    assert!(with.contains(&format!("<@{}>", OWNER + 9)));
}

#[test]
fn description_field_is_omitted_when_blank() {
    // Discord rejects a field with an empty value, so a description-less post
    // must drop the field rather than send an empty one.
    let blank =
        render(&DefaultTemplate::thread_embed(&post(&members(1), &[], ""), "Kilo"));
    assert!(!blank.contains("Description"));

    let filled = render(&DefaultTemplate::thread_embed(
        &post(&members(1), &[], "Mic required"),
        "Kilo",
    ));
    assert!(filled.contains("Description"));
    assert!(filled.contains("Mic required"));
}

#[test]
fn joined_field_survives_the_last_member_leaving() {
    // lfg DS-2: `leave` deletes the last `lfg_fireteam` row with no guard, so the
    // post is re-rendered with an empty roster. Discord requires an embed field
    // `value` of 1-1024 characters, so an empty join list must still emit a
    // non-empty value or the follow-up edit 400s and the post renders forever.
    let embed = DefaultTemplate::thread_embed(&post(&[], &[], ""), "Kilo");

    let (name, value) = fields(&embed)
        .into_iter()
        .find(|(name, _)| name.starts_with("Joined:"))
        .expect("the joined field is always emitted");

    assert_eq!(name, "Joined: 0/6");
    assert!(!value.is_empty(), "empty fireteam rendered an empty field value");
}

#[test]
fn no_embed_field_is_ever_emitted_with_an_empty_value() {
    // The invariant behind DS-2, swept across the shapes a post degrades through:
    // an empty roster, a blank description and no alternates each individually
    // drop a field's content to nothing.
    let shapes = [
        post(&[], &[], ""),
        post(&[], &[OWNER + 9], ""),
        post(&members(1), &[], ""),
        post(&members(6), &[OWNER + 9], "Mic required"),
    ];

    for row in &shapes {
        let embeds = [
            DefaultTemplate::thread_embed(row, "Kilo"),
            DefaultTemplate::message_embed(row, "Kilo", ThreadId::new(THREAD)),
        ];

        for embed in &embeds {
            for (name, value) in fields(embed) {
                assert!(
                    (1..=1024).contains(&value.chars().count()),
                    "field `{name}` has an illegal value length: {value:?}",
                );
            }
        }
    }
}

#[test]
fn only_the_scheduled_message_links_back_to_the_thread() {
    let row = post(&members(2), &[], "");

    let thread = render(&DefaultTemplate::thread_embed(&row, "Kilo"));
    assert!(
        !thread.contains("Event Thread"),
        "the in-thread embed must not link to itself",
    );

    let message =
        render(&DefaultTemplate::message_embed(&row, "Kilo", ThreadId::new(THREAD)));
    assert!(message.contains("Event Thread"));
    assert!(message.contains(&format!("<#{THREAD}>")));
}

#[test]
fn join_announcements_distinguish_fireteam_from_alternate() {
    let user = UserId::new(OWNER);

    let joined = Announcement::Joined { user, alternative: false }.to_string();
    assert_eq!(joined, format!("<@{OWNER}> joined the fireteam"));

    let alternate = Announcement::Joined { user, alternative: true }.to_string();
    assert_eq!(alternate, format!("<@{OWNER}> joined as an alternative"));

    assert_eq!(
        Announcement::Left(user).to_string(),
        format!("<@{OWNER}> left the fireteam"),
    );
}
