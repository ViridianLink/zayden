use std::fs;

use palworld::breeding::{BreedingIndex, combi_child};
use palworld::model::{Element, Pal};
use palworld::parse::pal_from_raw;
use palworld::transport::{BreedingMap, RawPal};
use palworld::typechart::{strong_against, weak_to};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

fn pals() -> Vec<Pal> {
    let raw: Vec<RawPal> =
        serde_json::from_str(&fixture("paldex_pals.json")).unwrap_or_default();
    raw.into_iter().map(pal_from_raw).collect()
}

fn index() -> BreedingIndex {
    let map: BreedingMap =
        serde_json::from_str(&fixture("paldex_breeding.json")).unwrap_or_default();
    BreedingIndex::from_map(map)
}

#[test]
fn forward_breed_is_deterministic_and_order_independent() {
    let index = index();
    assert_eq!(index.breed("001", "002"), Some("001"));
    // Parent order must not matter.
    assert_eq!(index.breed("002", "001"), index.breed("001", "002"));
}

#[test]
fn reverse_breed_lists_known_pair() {
    let index = index();
    let pairs = index.breed_for("001");
    assert!(!pairs.is_empty());
    assert!(pairs.iter().any(|p| {
        (p.a == "001" && p.b == "002") || (p.a == "002" && p.b == "001")
    }));
}

#[test]
fn legendaries_are_flagged_as_unique_combos() {
    let index = index();
    // Jetragon (111) and Frostallion (110) each have exactly one fixed combo.
    assert!(index.is_unique_child("111"));
    assert!(index.is_unique_child("110"));
    // Lamball is bred many ways — not a special combination.
    assert!(!index.is_unique_child("001"));
}

#[test]
fn combi_formula_matches_the_map_for_most_pairs() {
    let pals = pals();
    let index = index();

    let (mut total, mut agree) = (0_u32, 0_u32);
    for a in &pals {
        for b in &pals {
            if a.key > b.key {
                continue;
            }
            let Some(mapped) = index.breed(&a.key, &b.key) else { continue };
            let Some(formula) = combi_child(a, b, &pals) else { continue };
            total += 1;
            if formula.key == mapped {
                agree += 1;
            }
        }
    }

    assert!(total > 5000, "expected a large sample, got {total}");
    // The map is authoritative; the formula is an approximation that agrees on
    // the large majority of pairs (special combos and low-rank ties diverge).
    let rate = f64::from(agree) / f64::from(total);
    assert!(rate > 0.75, "formula/map agreement too low: {rate:.3}");
}

#[test]
fn type_chart_relationships_are_consistent() {
    assert!(strong_against(Element::Fire).contains(&Element::Grass));
    assert!(strong_against(Element::Water).contains(&Element::Fire));
    // Fire is weak to Water (Water is strong against Fire).
    assert_eq!(weak_to(Element::Fire), vec![Element::Water]);
    // Neutral attacks nothing for bonus damage.
    assert!(strong_against(Element::Neutral).is_empty());
    // Neutral is only weak to Dark.
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
