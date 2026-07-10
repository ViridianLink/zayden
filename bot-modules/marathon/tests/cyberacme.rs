//! `CyberAcme` parse tests against live-captured fixtures (2026-07-09) plus an
//! opt-in live smoke test (`--ignored`) that re-verifies the API schema.

use std::fs;

use marathon::parse;
use serde_json::Value;

fn load(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

#[test]
fn maps_weapon_stats_from_item_variant() {
    let envelope = load("cyberacme_item_brrt_smg");
    let item = envelope.get("item").expect("fixture has item");
    let weapon = parse::cyberacme_item_to_weapon("brrt-smg", item);

    assert_eq!(weapon.slug, "brrt-smg");
    assert_eq!(weapon.name, "BRRT Smg");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Submachine Gun"));
    // `firepower` is the damage stat.
    assert_eq!(weapon.damage.as_deref(), Some("16.1"));
    assert_eq!(weapon.range.as_deref(), Some("16m"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("35"));
    assert!(weapon.thumbnail_url.is_some());
    // The full stat block is exposed for the embed.
    assert!(weapon.stats.iter().any(|s| s.name == "Accuracy" && s.value == "65.3"));
    assert!(weapon.stats.iter().any(|s| s.name == "Handling"));
    // supportedModSlots become attachment slots.
    assert!(weapon.attachment_slots.iter().any(|s| s.slot == "Optic"));
}

#[test]
fn maps_faction_contracts() {
    let envelope = load("cyberacme_faction_cyberacme");
    let faction = parse::cyberacme_faction_to_model("cyberacme", &envelope);

    assert_eq!(faction.slug, "cyberacme");
    assert!(!faction.name.is_empty());
    assert!(
        !faction.priority_contracts.is_empty(),
        "cyberacme faction fixture has contracts"
    );
    let first = faction.priority_contracts.first().unwrap();
    assert!(!first.name.is_empty());
    assert!(first.description.is_some());
}

#[test]
fn maps_runner_identity() {
    let envelope = load("cyberacme_runners");
    let runners = envelope.get("runners").and_then(Value::as_array).unwrap();
    let first = runners.first().unwrap();
    let slug = first.get("slug").and_then(Value::as_str).unwrap();
    let runner = parse::cyberacme_runner_to_model(slug, first);

    assert_eq!(runner.slug, slug);
    assert!(!runner.name.is_empty());
    assert!(runner.portrait_url.is_some());
}

/// Opt-in: `cargo test -p marathon --test cyberacme -- --ignored`. Proves the
/// live API still matches the parser's expectations.
#[tokio::test]
#[ignore = "hits the live CyberAcme API"]
async fn live_weapon_parses_non_empty() {
    let client = reqwest::Client::new();
    let ca = marathon::transport::CyberAcme::new(client);
    let item = ca.item("brrt-smg").await.expect("live item fetch");
    let weapon = parse::cyberacme_item_to_weapon("brrt-smg", &item);
    assert_eq!(weapon.name, "BRRT Smg");
    assert!(weapon.damage.is_some(), "live weapon should have firepower");
}
