//! Breeding + difficulty against the `PalCalc` fixtures (see `tests/fixtures/`,
//! captured 2026-07-13 from `tylercamp/palcalc`). These pals and combos include
//! post-1.0 content (Nyafia, the Yakushima slimes) that the legacy mlg404
//! source never had.

use std::collections::HashMap;
use std::fs;

use palworld::breeding::BreedingIndex;
use palworld::difficulty::{pair_difficulty, pal_difficulty};
use palworld::model::{Element, Pal};
use palworld::parse::pal_from_palcalc;
use palworld::transport::{parse_breeding, parse_pals};
use palworld::typechart::{strong_against, weak_to};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

fn pals() -> Vec<Pal> {
    parse_pals(&fixture("palcalc_db.json"))
        .unwrap_or_default()
        .into_iter()
        .map(pal_from_palcalc)
        .collect()
}

fn index() -> BreedingIndex {
    BreedingIndex::from_map(
        parse_breeding(&fixture("palcalc_breeding.json")).unwrap_or_default(),
    )
}

#[test]
fn roster_carries_current_pals_and_breeding_fields() {
    let pals = pals();
    let find = |key: &str| pals.iter().find(|p| p.key == key).expect("pal present");

    // Nyafia is Feybreak-era content absent from the old source.
    let nyafia = find("BadCatgirl");
    assert_eq!(nyafia.name, "Nyafia");
    assert_eq!(nyafia.rarity, Some(4));
    assert_eq!(nyafia.min_wild_level, Some(30));
    // BreedingPower maps onto breeding_rank so combos resolve.
    assert!(nyafia.breeding_rank.is_some());

    // A pal with no wild spawn carries None (obtainable only via breeding).
    assert_eq!(find("NightLady").min_wild_level, None);
}

#[test]
fn forward_breed_is_deterministic_and_order_independent() {
    let index = index();
    assert_eq!(index.breed("PinkCat", "Bastet_Ice"), Some("PinkCat"));
    assert_eq!(
        index.breed("Bastet_Ice", "PinkCat"),
        index.breed("PinkCat", "Bastet_Ice"),
    );
}

#[test]
fn reverse_breed_lists_known_pair() {
    let index = index();
    let pairs = index.breed_for("SheepBall");
    assert!(!pairs.is_empty());
    assert!(pairs.iter().any(|p| {
        (p.a == "YakushimaMonster001" && p.b == "NegativeKoala")
            || (p.a == "NegativeKoala" && p.b == "YakushimaMonster001")
    }));
}

#[test]
fn difficulty_ranks_common_pals_below_legendaries() {
    let pals = pals();
    let find = |key: &str| pals.iter().find(|p| p.key == key).expect("pal present");
    let lamball = pal_difficulty(find("SheepBall"));
    let bellanoir = pal_difficulty(find("NightLady"));
    let jetragon = pal_difficulty(find("JetDragon"));

    // Low-level common < high-level legendary < no-wild legendary.
    assert!(lamball < jetragon);
    assert!(jetragon < bellanoir);
}

#[test]
fn breed_for_sorts_easiest_combo_first() {
    let pals = pals();
    let index = index();
    let lookup: HashMap<&str, &Pal> =
        pals.iter().map(|p| (p.key.as_str(), p)).collect();

    let mut all = index.breed_for("SheepBall");
    all.sort_by_cached_key(|pair| {
        match (lookup.get(pair.a.as_str()), lookup.get(pair.b.as_str())) {
            (Some(a), Some(b)) => pair_difficulty(a, b),
            _ => (i64::MAX, i64::MAX),
        }
    });

    // Lamball × Lamball (both wild-level 1, rarity 1) is the cheapest route and
    // must come before any slime-based combo.
    let first = all.first().expect("at least one combo");
    assert_eq!(first.a, "SheepBall");
    assert_eq!(first.b, "SheepBall");
}

#[test]
fn type_chart_relationships_are_consistent() {
    assert!(strong_against(Element::Fire).contains(&Element::Grass));
    assert!(strong_against(Element::Water).contains(&Element::Fire));
    assert_eq!(weak_to(Element::Fire), vec![Element::Water]);
    assert!(strong_against(Element::Neutral).is_empty());
    assert_eq!(weak_to(Element::Neutral), vec![Element::Dark]);
}

#[test]
fn element_parse_tolerates_aliases() {
    assert_eq!(Element::parse("leaf"), Some(Element::Grass));
    assert_eq!(Element::parse("Lightning"), Some(Element::Electric));
    assert_eq!(Element::parse("earth"), Some(Element::Ground));
    assert_eq!(Element::parse("NEUTRAL"), Some(Element::Neutral));
    assert_eq!(Element::parse("nonsense"), None);
}
