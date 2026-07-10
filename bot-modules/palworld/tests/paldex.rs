//! Parse the live-captured Paldex fixtures (see `tests/fixtures/`, captured
//! 2026-07-10 from mlg404/palworld-paldex-api) into the domain model.

use std::collections::HashMap;
use std::fs;

use palworld::model::Element;
use palworld::parse::{item_from_raw, pal_from_raw, passive_from_raw};
use palworld::transport::{RawItem, RawPal, RawPassive};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn parses_lamball_from_paldex_fixture() {
    let raw: Vec<RawPal> =
        serde_json::from_str(&fixture("paldex_pals.json")).expect("valid pals json");
    assert_eq!(raw.len(), 137);

    let lamball = raw
        .into_iter()
        .map(pal_from_raw)
        .find(|p| p.key == "001")
        .expect("Lamball present");

    assert_eq!(lamball.name, "Lamball");
    assert_eq!(lamball.paldex_no, 1);
    assert_eq!(lamball.elements, vec![Element::Neutral]);
    assert!(lamball.child_eligible);
    assert_eq!(lamball.breeding_rank, Some(1470));
    assert!(lamball.stats.is_some_and(|s| s.hp == 70));
    assert!(lamball.drops.iter().any(|d| d == "wool"));
    assert!(lamball.partner_skill.is_some());
    assert!(lamball.description.is_some());
}

#[test]
fn parses_items_and_passives() {
    let raw_items: Vec<RawItem> =
        serde_json::from_str(&fixture("paldex_item.json")).expect("valid item json");
    let items: Vec<_> = raw_items.into_iter().map(item_from_raw).collect();
    assert!(items.iter().any(|i| i.name == "Gold Coin"));

    let raw_passives: HashMap<String, RawPassive> =
        serde_json::from_str(&fixture("paldex_passive_skills.json"))
            .expect("valid passives json");
    let passives: Vec<_> = raw_passives
        .into_iter()
        .map(|(k, v)| passive_from_raw(k, v))
        .collect();
    let artisan =
        passives.iter().find(|p| p.key == "artisan").expect("artisan present");
    assert_eq!(artisan.name, "Artisan");
    assert_eq!(artisan.tier, 3);
    assert!(artisan.positive.is_some());
}
