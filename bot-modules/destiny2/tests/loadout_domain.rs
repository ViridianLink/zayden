//! Behaviour locked by the normalized loadout DB layer (POST.md #3/#4):
//! - The loadout render pipeline stores emoji-cache *key* strings and enum
//!   domain values; these enums map onto the `destiny2_*` Postgres enum types,
//!   so their `Display`/`FromStr` surfaces must stay stable for the round trip
//!   through the database to be lossless.
//! - `Element::key` is the subclass emoji-cache key and the subclass button id.
//! - `ArmourSlot::render_order` is the fixed order armour is stored/rendered in.

use std::str::FromStr;

use destiny2::loadouts::{ArmourSlot, Class, Element, Mode, StatKind};

#[test]
fn class_display_fromstr_round_trip() {
    for class in [Class::Hunter, Class::Titan, Class::Warlock] {
        assert_eq!(Class::from_str(&class.to_string()), Ok(class));
    }
}

#[test]
fn class_parses_lowercase_subcommand_names() {
    assert_eq!(Class::from_str("hunter"), Ok(Class::Hunter));
    assert_eq!(Class::from_str("titan"), Ok(Class::Titan));
    assert_eq!(Class::from_str("warlock"), Ok(Class::Warlock));
    assert!(Class::from_str("gunslinger").is_err());
}

#[test]
fn element_display_fromstr_round_trip() {
    for element in [
        Element::Arc,
        Element::Solar,
        Element::Void,
        Element::Strand,
        Element::Stasis,
        Element::Prismatic,
    ] {
        assert_eq!(Element::from_str(&element.to_string()), Ok(element));
    }
}

#[test]
fn element_key_is_lowercase_display() {
    assert_eq!(Element::Arc.key(), "arc");
    assert_eq!(Element::Prismatic.key(), "prismatic");
}

#[test]
fn stat_kind_keys_are_lowercase() {
    assert_eq!(StatKind::Health.to_string(), "health");
    assert_eq!(StatKind::Super.to_string(), "super");
    assert_eq!(StatKind::Weapons.to_string(), "weapons");
}

#[test]
fn mode_display_forms() {
    assert_eq!(Mode::All.to_string(), "All");
    assert_eq!(Mode::PvE.to_string(), "PvE");
    assert_eq!(Mode::PvP.to_string(), "PvP");
}

#[test]
fn armour_render_order_is_head_to_class_item() {
    assert_eq!(ArmourSlot::render_order(), [
        ArmourSlot::Helmet,
        ArmourSlot::Arms,
        ArmourSlot::Chest,
        ArmourSlot::Legs,
        ArmourSlot::ClassItem,
    ]);
}
