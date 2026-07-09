//! Integration tests: parse real, live-captured fixtures (see
//! `bot-modules/marathon/tests/fixtures/`, captured 2026-07-08) through
//! `gql`/`parse` and confirm they deserialize into sane `model.rs` structs,
//! including graceful handling of data a fixture doesn't contain.

use std::fs;

use marathon::model::MapStatus;
use marathon::{gql, parse};
use serde_json::Value;

/// Returns `Value::Null` on any read/parse failure rather than panicking here - the
/// caller (a `#[test]` fn) asserts on the loaded shape and gets a clear failure at
/// the actual call site.
fn load_fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(Value::Null)
}

#[test]
fn parses_weapon_from_mobalytics_fixture() {
    let marathon_state = load_fixture("mobalytics_weapon_d54");
    let doc =
        gql::find_struct_document(&marathon_state, "weapons/d54-battle-pistol")
            .expect("fixture should contain the weapon document");

    let weapon = parse::parse_weapon("d54-battle-pistol", doc);

    assert_eq!(weapon.slug, "d54-battle-pistol");
    assert_eq!(weapon.name, "D54 Battle Pistol");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Pistol"));
    assert_eq!(weapon.damage.as_deref(), Some("16"));
    assert_eq!(weapon.fire_rate.as_deref(), Some("1140 RPM"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("21"));
    assert_eq!(weapon.reload_speed.as_deref(), Some("2.69s"));
    assert_eq!(weapon.range.as_deref(), Some("29m"));
    assert!(weapon.description.is_some());
    assert!(!weapon.stats.is_empty());

    // The fixture's "Compatible Mods" section is a "Coming soon..." placeholder -
    // this must degrade to an empty Vec rather than fabricating attachment data.
    assert!(weapon.attachment_slots.is_empty());
}

#[test]
fn parses_runner_from_mobalytics_fixture() {
    let marathon_state = load_fixture("mobalytics_runner_assassin");
    let doc = gql::find_struct_document(&marathon_state, "runners/assassin")
        .expect("fixture should contain the runner document");

    let runner = parse::parse_runner("assassin", doc);

    assert_eq!(runner.slug, "assassin");
    assert!(runner.name.contains("Assassin"));
    assert!(!runner.stats.is_empty());
    assert!(
        runner.stats.iter().any(|s| s.name == "Heat Capacity" && s.value == "10")
    );

    // 4 named abilities/traits: Smoke Screen, Active Camo, Shadow Dive, Shroud.
    assert_eq!(runner.abilities.len(), 4);
    let smoke_screen = runner
        .abilities
        .iter()
        .find(|a| a.name == "Smoke Screen")
        .expect("Smoke Screen ability should be parsed");
    assert_eq!(smoke_screen.ability_type.as_deref(), Some("Prime Ability"));
    assert_eq!(smoke_screen.cooldown_seconds, Some(163));
    assert!(
        smoke_screen
            .description
            .as_deref()
            .unwrap_or_default()
            .contains("smoke disc")
    );

    assert!(runner.cores.contains(&"Breathing Space".to_string()));
}

#[test]
fn parses_faction_listing_from_mobalytics_fixture() {
    let marathon_state = load_fixture("mobalytics_factions_listing");
    let doc = gql::find_struct_document(&marathon_state, "factions")
        .expect("fixture should contain the factions listing document");

    let factions = parse::parse_faction_listing(doc);

    assert_eq!(factions.len(), 6);
    let cyberacme = factions
        .iter()
        .find(|f| f.slug == "cyberacme")
        .expect("Cyberacme faction should be parsed");
    assert_eq!(cyberacme.name, "Cyberacme");
    // The listing page only has name/slug - contract/upgrade data lives on the
    // per-faction page.
    assert!(cyberacme.priority_contracts.is_empty());
    assert!(cyberacme.upgrades.is_empty());
}

#[test]
fn parses_map_from_mobalytics_fixture() {
    let marathon_state = load_fixture("mobalytics_map_perimeter");
    let doc = gql::find_struct_document(&marathon_state, "maps/perimeter")
        .expect("fixture should contain the map document");

    let map = parse::parse_map("perimeter", doc);

    assert_eq!(map.slug, "perimeter");
    assert_eq!(map.name, "Perimeter");
    // Confirmed absent from the source - see Design §Unverified Assumptions 5.
    assert_eq!(map.status, None::<MapStatus>);
    assert!(!map.extractions.is_empty());
    assert!(map.extractions.iter().any(|l| l.name.contains("Exfil")));
    assert!(
        map.map_image_url
            .as_deref()
            .is_some_and(|url| url.contains("map-perimeter-starter.png"))
    );
}

#[test]
fn parses_build_from_mobalytics_ug_fixture() {
    let marathon_state = load_fixture("mobalytics_build_wallzer_thief");
    let doc = gql::find_ug_document(
        &marathon_state,
        "builds",
        "wallzer-greed-is-good-thief",
    )
    .expect("fixture should contain the build document");

    let build = parse::parse_build("wallzer-greed-is-good-thief", doc);

    assert_eq!(build.slug, "wallzer-greed-is-good-thief");
    assert_eq!(build.name, "Greed is Good Thief");
    assert_eq!(build.shell.as_deref(), Some("Thief"));
    assert!(build.gear.iter().any(|g| g.contains("WSTR Combat Shotgun")));
    // Cradle content wasn't confirmed present this session - must degrade to None,
    // not a guess.
    assert_eq!(build.cradle_focus, None);
}

#[test]
fn marathondb_weapon_fallback_maps_known_fields() {
    let data = load_fixture("marathondb_weapon");
    let payload = data.get("data").expect("fixture should have a data envelope");

    let weapon = parse::marathondb_weapon_to_model("d54-battle-pistol", payload);

    assert_eq!(weapon.name, "D54 BATTLE PISTOL");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Standard Pistol"));
    assert_eq!(weapon.damage.as_deref(), Some("16"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("21"));
    assert!(weapon.attachment_slots.is_empty());
}

#[test]
fn marathondb_runner_fallback_maps_known_fields() {
    let data = load_fixture("marathondb_runner");
    let payload = data.get("data").expect("fixture should have a data envelope");

    let runner = parse::marathondb_runner_to_model("assassin", payload);

    assert_eq!(runner.name, "Assassin");
    assert_eq!(runner.role.as_deref(), Some("Shadow Agent"));
    assert_eq!(runner.abilities.len(), 5);
    assert!(runner.cores.is_empty());
}

#[test]
fn marathondb_contracts_fallback_groups_by_faction() {
    let data = load_fixture("marathondb_contracts_active");
    let contracts = data
        .get("data")
        .and_then(Value::as_array)
        .expect("fixture should have a data array")
        .clone();

    let factions = parse::marathondb_contracts_to_factions(&contracts);

    assert_eq!(factions.len(), 3);
    let nucaloric = factions
        .iter()
        .find(|f| f.slug == "nucaloric")
        .expect("Nucaloric faction should be present");
    assert_eq!(nucaloric.priority_contracts.len(), 1);
    assert!(nucaloric.upgrades.is_empty());
}

#[test]
fn missing_document_degrades_to_none_rather_than_panicking() {
    let marathon_state = load_fixture("mobalytics_weapon_d54");
    assert!(
        gql::find_struct_document(&marathon_state, "weapons/does-not-exist")
            .is_none()
    );
    assert!(
        gql::find_ug_document(&marathon_state, "builds", "does-not-exist").is_none()
    );
}
