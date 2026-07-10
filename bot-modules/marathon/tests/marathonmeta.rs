//! `MarathonMeta` parse tests against live-captured rendered HTML (2026-07-10).
//! The fixtures are the fully hydrated pages `FlareSolverr` returns, from which
//! the parsers read the stat grids with `scraper`.

use std::fs;

use marathon::parse;

fn load(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn maps_weapon_identity_and_stats() {
    let html = load("marathonmeta_weapon_m77.html");
    let weapon = parse::marathonmeta_html_to_weapon("m77-assault-rifle", &html);

    assert_eq!(weapon.slug, "m77-assault-rifle");
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Assault Rifle"));
    assert_eq!(weapon.description.as_deref(), Some("Ballistic assault rifle."));
    // Scalar fields are normalised to bare numbers so they line up with the
    // other sources (Rate of Fire renders as "450 RPM" on the page).
    assert_eq!(weapon.damage.as_deref(), Some("16"));
    assert_eq!(weapon.range.as_deref(), Some("46"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("24"));
    assert_eq!(weapon.fire_rate.as_deref(), Some("450"));
    assert!(weapon.thumbnail_url.is_some());
    // Both the hero grid (Range) and detail grid (Reload Speed) are unioned,
    // with display values kept verbatim.
    assert!(weapon.stats.iter().any(|s| s.name == "Range" && s.value == "46"));
    assert!(
        weapon
            .stats
            .iter()
            .any(|s| s.name == "Rate of Fire" && s.value == "450 RPM")
    );
    assert!(weapon.stats.iter().any(|s| s.name == "Recoil"));
    // The sidebar's list of every other weapon must not leak into the stats.
    assert!(!weapon.stats.iter().any(|s| s.name == "ARES RG"));
}

#[test]
fn maps_runner_identity_and_base_stats() {
    let html = load("marathonmeta_runner_rook.html");
    let runner = parse::marathonmeta_html_to_runner("rook", &html);

    assert_eq!(runner.slug, "rook");
    assert_eq!(runner.name, "Rook");
    assert_eq!(runner.role.as_deref(), Some("Scavenger"));
    // Full description comes from the page paragraph, not the truncated meta.
    let description = runner.description.expect("runner description");
    assert!(description.starts_with("Other players aren't notified"));
    assert!(description.ends_with("with zero stakes."));
    assert!(runner.portrait_url.is_some());
    // Base stats are captured; non-numeric rows (the site logo) are filtered.
    assert!(runner.stats.iter().any(|s| s.name == "Loot Speed" && s.value == "20"));
    assert!(runner.stats.iter().any(|s| s.name == "Agility"));
    assert!(!runner.stats.iter().any(|s| s.name == "MARATHON"));
}

/// Opt-in: `MARATHONMETA_FLARESOLVERR=http://10.0.3.119:8191/v1 \
/// cargo test -p marathon --test marathonmeta -- --ignored`. Proves the live
/// page's rendered DOM still matches the parser.
#[tokio::test]
#[ignore = "hits the live MarathonMeta site through FlareSolverr"]
async fn live_weapon_parses_non_empty() {
    let Ok(flaresolverr) = std::env::var("MARATHONMETA_FLARESOLVERR") else {
        panic!("set MARATHONMETA_FLARESOLVERR to the FlareSolverr /v1 endpoint");
    };
    let client = reqwest::Client::new();
    let marathonmeta = marathon::transport::MarathonMeta::new(client, flaresolverr);
    let html = marathonmeta.weapon("m77-assault-rifle").await.expect("live fetch");
    let weapon = parse::marathonmeta_html_to_weapon("m77-assault-rifle", &html);
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert!(weapon.damage.is_some(), "live weapon should carry stats");
}
