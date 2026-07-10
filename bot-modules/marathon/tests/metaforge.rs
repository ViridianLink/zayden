//! `MetaForge` parse tests against live-captured marker data (2026-07-10).
//!
//! `MetaForge` is a `SvelteKit` site whose curated map pins live in a Supabase
//! (`PostgREST`) table, `marathon_map_data`. Each row is a coordinate marker
//! tagged with a `category`/`subcategory`; the fixture is the perimeter map's
//! rows (`select=category,subcategory,instance_name`) verbatim.

use std::fs;

use marathon::parse;
use serde_json::Value;

fn load(name: &str) -> Vec<Value> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&fs::read_to_string(&path).unwrap_or_default())
        .unwrap_or_default()
}

fn has(list: &[impl AsRef<str>], name: &str) -> bool {
    list.iter().any(|n| n.as_ref().eq_ignore_ascii_case(name))
}

#[test]
fn buckets_markers_by_category() {
    let rows = load("metaforge_perimeter_markers.json");
    let map = parse::metaforge_markers_to_map("perimeter", &rows);

    assert_eq!(map.slug, "perimeter");
    assert_eq!(map.name, "Perimeter");

    // `locations/*-exfil` markers become extractions.
    let exfils: Vec<&str> = map.extractions.iter().map(|l| l.name.as_str()).collect();
    assert!(has(&exfils, "Guarded Exfil"));
    assert!(has(&exfils, "Crew Exfil"));
    assert!(has(&exfils, "Final Exfil"));

    // `locations/locked-room` and `/vault` become keycard rooms, keeping their
    // specific instance names.
    let rooms: Vec<&str> =
        map.keycard_rooms.iter().map(|r| r.name.as_str()).collect();
    assert!(has(&rooms, "Station Security Office"));

    // `activities/*` markers become event spawns.
    let events: Vec<&str> =
        map.event_spawns.iter().map(|l| l.name.as_str()).collect();
    assert!(has(&events, "Supply Drop"));
    assert!(has(&events, "Wraith Warden"));

    // Remaining `locations` markers are plain POIs.
    let pois: Vec<&str> = map.pois.iter().map(|p| p.name.as_str()).collect();
    assert!(has(&pois, "Player Spawn"));
}

#[test]
fn skips_loot_and_objective_categories() {
    let rows = load("metaforge_perimeter_markers.json");
    let map = parse::metaforge_markers_to_map("perimeter", &rows);

    // Container, enemy, nature and quest markers have no home in the model and
    // must not leak into any bucket.
    let every: Vec<String> = map
        .pois
        .iter()
        .map(|p| p.name.clone())
        .chain(map.extractions.iter().map(|l| l.name.clone()))
        .chain(map.event_spawns.iter().map(|l| l.name.clone()))
        .chain(map.keycard_rooms.iter().map(|r| r.name.clone()))
        .collect();

    assert!(!has(&every, "Folio"), "container marker leaked in");
    assert!(!has(&every, "Trunk"), "container marker leaked in");
    assert!(!has(&every, "Uesc Recruit"), "enemy marker leaked in");
    // Quest steps carry verbose descriptions; none should appear.
    assert!(
        !every.iter().any(|n| n.contains("Download")),
        "quest-step text leaked in"
    );
}

#[test]
fn dedups_repeated_markers() {
    let rows = load("metaforge_perimeter_markers.json");
    let map = parse::metaforge_markers_to_map("perimeter", &rows);

    // "Guarded Exfil" appears many times on the map; the bucket keeps one.
    let guarded = map
        .extractions
        .iter()
        .filter(|l| l.name.eq_ignore_ascii_case("Guarded Exfil"))
        .count();
    assert_eq!(guarded, 1);
}

/// Opt-in live check:
/// `cargo test -p marathon --test metaforge -- --ignored`.
/// The Supabase gateway is Cloudflare-fronted but not challenge-gated, so this
/// needs no `FlareSolverr`.
#[tokio::test]
#[ignore = "hits the live MetaForge Supabase gateway"]
async fn live_markers_parse_non_empty() {
    let client = reqwest::Client::new();
    let metaforge = marathon::transport::MetaForge::new(client);

    let rows = metaforge.map_markers("perimeter").await.expect("live markers");
    assert!(!rows.is_empty(), "live query should return markers");

    let map = parse::metaforge_markers_to_map("perimeter", &rows);
    assert!(!map.extractions.is_empty(), "live map should carry extractions");
}
