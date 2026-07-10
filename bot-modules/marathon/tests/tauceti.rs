//! `TauCeti` parse tests against live-captured fixtures (2026-07-10). The
//! fixtures are the resolved React Flight objects `flight_object_by_slug`
//! extracts from each detail page.

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
fn maps_weapon_stats_from_flight_object() {
    let item = load("tauceti_weapon_m77");
    let weapon = parse::tauceti_item_to_weapon("m77-assault-rifle", &item);

    assert_eq!(weapon.slug, "m77-assault-rifle");
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Assault Rifle"));
    assert_eq!(weapon.damage.as_deref(), Some("16"));
    assert_eq!(weapon.range.as_deref(), Some("46"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("24"));
    assert_eq!(weapon.fire_rate.as_deref(), Some("450"));
    assert!(weapon.thumbnail_url.is_some());
    assert!(weapon.description.is_some());
    // The full stat block is surfaced for the embed.
    assert!(weapon.stats.iter().any(|s| s.name == "Accuracy"));
    assert!(weapon.stats.iter().any(|s| s.name == "Firing Mode"));
    // activeModSlots become attachment slots.
    let slots: Vec<&str> =
        weapon.attachment_slots.iter().map(|s| s.slot.as_str()).collect();
    assert_eq!(slots, ["Chip", "Magazine", "Grip"]);
}

#[test]
fn maps_runner_identity_and_stats() {
    let value = load("tauceti_runner_rook");
    let runner = parse::tauceti_runner_to_model("rook", &value);

    assert_eq!(runner.slug, "rook");
    assert_eq!(runner.name, "Rook");
    assert_eq!(runner.role.as_deref(), Some("Scavenger"));
    assert!(runner.portrait_url.is_some());
    // baseStats camelCase keys are humanized.
    assert!(runner.stats.iter().any(|s| s.name == "Melee Damage"));
    // abilities are flattened out of the nested frame variants.
    assert!(!runner.abilities.is_empty());
    assert!(runner.abilities.iter().all(|a| !a.name.is_empty()));
}

#[test]
fn maps_faction_identity() {
    let value = load("tauceti_faction_mida");
    let faction = parse::tauceti_faction_to_model("mida", &value);

    assert_eq!(faction.slug, "mida");
    assert_eq!(faction.name, "MIDA");
    // Contracts/upgrades are unpopulated pre-launch but must not panic.
    assert!(faction.priority_contracts.is_empty());
}

/// Opt-in: `TAUCETI_FLARESOLVERR=http://10.0.3.119:8191/v1 \
/// cargo test -p marathon --test tauceti -- --ignored`. Proves the live page's
/// React Flight schema still matches the parser.
#[tokio::test]
#[ignore = "hits the live TauCeti site through FlareSolverr"]
async fn live_weapon_parses_non_empty() {
    let Ok(flaresolverr) = std::env::var("TAUCETI_FLARESOLVERR") else {
        panic!("set TAUCETI_FLARESOLVERR to the FlareSolverr /v1 endpoint");
    };
    let client = reqwest::Client::new();
    let tauceti = marathon::transport::TauCeti::new(client, flaresolverr);
    let item = tauceti.weapon("m77-assault-rifle").await.expect("live fetch");
    let weapon = parse::tauceti_item_to_weapon("m77-assault-rifle", &item);
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert!(weapon.damage.is_some(), "live weapon should carry stats");
}
