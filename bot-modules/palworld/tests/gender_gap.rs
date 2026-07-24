//! `gender_gap` - the owned-roster check that reports which gender a breeding
//! pair still needs (same-species needs the opposite gender; cross-species
//! needs one of each), or `None` when the pair is already breedable / unowned.

use std::collections::HashMap;

use palworld::commands::gender_gap;

fn owned<'a>(entries: &[(&'a str, (bool, bool))]) -> HashMap<&'a str, (bool, bool)> {
    entries.iter().copied().collect()
}

fn id(key: &str) -> String {
    key.to_string()
}

#[test]
fn same_species_single_gender_needs_opposite() {
    let g = owned(&[("A", (true, false))]);
    assert_eq!(
        gender_gap("A", "A", &g, &id),
        Some("another **A** (female)".to_string())
    );

    let g = owned(&[("A", (false, true))]);
    assert_eq!(
        gender_gap("A", "A", &g, &id),
        Some("another **A** (male)".to_string())
    );
}

#[test]
fn same_species_both_genders_has_no_gap() {
    let g = owned(&[("A", (true, true))]);
    assert_eq!(gender_gap("A", "A", &g, &id), None);
}

#[test]
fn different_species_same_gender_needs_opposite() {
    let g = owned(&[("A", (true, false)), ("B", (true, false))]);
    assert_eq!(
        gender_gap("A", "B", &g, &id),
        Some("an opposite-gender **A** or **B**".to_string())
    );
}

#[test]
fn different_species_compatible_has_no_gap() {
    let g = owned(&[("A", (true, false)), ("B", (false, true))]);
    assert_eq!(gender_gap("A", "B", &g, &id), None);
}

#[test]
fn unowned_parent_yields_no_gap() {
    // A parent that will be caught or bred is not an owned-gender problem.
    let g = owned(&[("A", (true, false))]);
    assert_eq!(gender_gap("A", "B", &g, &id), None);
    assert_eq!(gender_gap("X", "Y", &g, &id), None);
}
