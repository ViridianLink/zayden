use deadlock::db::patches::PatchRow;
use deadlock::diff::{Change, HeroDiff, PatchDiff, Presence};
use deadlock::embeds::patch_component;
use jiff::Timestamp;
use serde_json::Value;
use zayden_core::limits::message::COMPONENTS_TEXT as MAX_TEXT;

const TITLE: &str = "Update 14 October";
const LINK: &str = "https://forums.playdeadlock.com/threads/update-14-october.1/";

fn patch(diff: Option<PatchDiff>) -> PatchRow {
    PatchRow {
        guid: "forum-1".to_owned(),
        title: TITLE.to_owned(),
        link: LINK.to_owned(),
        published_at: Timestamp::UNIX_EPOCH,
        detected_at: Timestamp::UNIX_EPOCH,
        diff,
    }
}

fn change(label: String, old: &str, new: &str) -> Change {
    Change { label, old: Some(old.to_owned()), new: Some(new.to_owned()) }
}

fn hero(stats: Vec<Change>) -> HeroDiff {
    HeroDiff {
        hero_id: 1,
        name: "Infernus".to_owned(),
        presence: Presence::Changed,
        stats,
        weapon: Vec::new(),
        abilities: Vec::new(),
    }
}

fn text_len(value: &Value) -> usize {
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, inner)| {
                if key == "content" {
                    inner.as_str().map_or(0, |text| text.chars().count())
                } else {
                    text_len(inner)
                }
            })
            .sum(),
        Value::Array(items) => items.iter().map(text_len).sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}

#[test]
fn the_card_lists_the_title_link_and_hero_changes() {
    let diff = PatchDiff {
        heroes: vec![hero(vec![change("Max health".to_owned(), "830", "850")])],
        items: Vec::new(),
    };

    let json = serde_json::to_string(&patch_component(&patch(Some(diff)), MAX_TEXT))
        .unwrap();

    assert!(json.contains(TITLE));
    assert!(json.contains(LINK));
    assert!(json.contains("### Infernus"));
    assert!(json.contains("- Max health: 830 → 850"));
    assert!(!json.contains("more changes"));
}

#[test]
fn a_patch_without_changes_says_so() {
    let json =
        serde_json::to_string(&patch_component(&patch(None), MAX_TEXT)).unwrap();

    assert!(json.contains("No hero or item stat changes detected."));
}

#[test]
fn a_huge_diff_is_truncated_within_the_text_budget() {
    let stats = (0..500)
        .map(|index| change(format!("Stat number {index}"), "100", "200"))
        .collect();
    let diff = PatchDiff { heroes: vec![hero(stats)], items: Vec::new() };

    let component =
        serde_json::to_value(patch_component(&patch(Some(diff)), MAX_TEXT)).unwrap();
    let json = component.to_string();

    assert!(json.contains(TITLE));
    assert!(json.contains("### Infernus"));
    assert!(json.contains("more changes in the official notes."));
    assert!(text_len(&component) <= MAX_TEXT);
}
