//! Marathon Guide parse tests against live-captured HTML (2026-07-10).
//!
//! Marathon Guide is a statically prerendered Angular site: every weapon is
//! rendered onto a single `/weapons/card` page, and each runner has its own
//! `/runners/<slug>` detail page. The fixtures are those pages verbatim.

use std::fs;

use marathon::parse;

fn load(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn maps_weapon_card_identity_and_stats() {
    let page = load("marathonguide_weapons_card.html");
    let weapon = parse::marathonguide_html_to_weapon("m77-assault-rifle", &page)
        .expect("m77 card present");

    assert_eq!(weapon.slug, "m77-assault-rifle");
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert_eq!(weapon.weapon_type.as_deref(), Some("Assault Rifle"));
    assert_eq!(weapon.ammo_type.as_deref(), Some("Light Rounds"));
    assert!(
        weapon
            .description
            .as_deref()
            .is_some_and(|d| d.starts_with("Ballistic assault rifle"))
    );
    assert!(
        weapon
            .thumbnail_url
            .as_deref()
            .is_some_and(|u| u.ends_with("/weapons/m77-assault-rifle-180x135.png"))
    );

    // Marathon Guide only exposes Range and Magazine as bare scalars; damage and
    // fire rate are not on the card, so those stay `None` for the merge.
    assert_eq!(weapon.range.as_deref(), Some("46"));
    assert_eq!(weapon.magazine_size.as_deref(), Some("24"));
    assert_eq!(weapon.damage, None);
    assert_eq!(weapon.fire_rate, None);

    // Stat cells are kept verbatim (units and all).
    let stat = |name: &str| {
        weapon.stats.iter().find(|s| s.name == name).map(|s| s.value.as_str())
    };
    assert_eq!(stat("Range"), Some("46m"));
    assert_eq!(stat("Mag"), Some("24"));
    assert_eq!(stat("DPS"), Some("120"));
    assert_eq!(stat("Zoom"), Some("1.2x"));
    assert!(stat("Handling").is_some());

    // The mod slots are captured as empty attachment slots.
    let slots: Vec<&str> =
        weapon.attachment_slots.iter().map(|s| s.slot.as_str()).collect();
    assert!(slots.contains(&"Chip"));
    assert!(slots.contains(&"Magazine"));
    assert!(slots.contains(&"Grip"));
    assert!(weapon.attachment_slots.iter().all(|s| s.attachment.is_none()));
}

#[test]
fn distinguishes_weapons_sharing_an_image() {
    let page = load("marathonguide_weapons_card.html");

    // "M77 Omnipositor" reuses the M77 Assault Rifle image, so identity must
    // come from the slugified name, not the image src.
    let omni = parse::marathonguide_html_to_weapon("m77-omnipositor", &page)
        .expect("omnipositor card present");
    assert_eq!(omni.name, "M77 Omnipositor");

    // A slug no card matches yields `None`, letting the client skip the source.
    assert!(parse::marathonguide_html_to_weapon("does-not-exist", &page).is_none());
}

#[test]
fn maps_runner_abilities_and_summary_tags() {
    let page = load("marathonguide_runner_rook.html");
    let runner = parse::marathonguide_html_to_runner("rook", &page);

    assert_eq!(runner.slug, "rook");
    assert_eq!(runner.name, "Rook");
    // Marathon Guide labels Rook's role "Opportunist" (its own taxonomy); the
    // merge reconciles that against the other sources.
    assert_eq!(runner.role.as_deref(), Some("Opportunist"));
    assert!(
        runner
            .portrait_url
            .as_deref()
            .is_some_and(|u| u.ends_with("/portraits/rook-150x230.png"))
    );
    assert!(runner.cores.is_empty());

    // Prime and Tactical abilities carry their type and description.
    let prime = runner
        .abilities
        .iter()
        .find(|a| a.name == "Recuperation")
        .expect("prime ability");
    assert_eq!(prime.ability_type.as_deref(), Some("Prime Ability"));
    assert!(prime.description.as_deref().is_some_and(|d| d.contains("repair")));

    let tactical = runner
        .abilities
        .iter()
        .find(|a| a.name == "Signal Mask")
        .expect("tactical ability");
    assert_eq!(tactical.ability_type.as_deref(), Some("Tactical Ability"));

    // Origin and Tech summary tags surface as stats; Role is promoted out.
    let stat = |name: &str| {
        runner.stats.iter().find(|s| s.name == name).map(|s| s.value.as_str())
    };
    assert_eq!(stat("Origin"), Some("Lunar work release"));
    assert_eq!(stat("Tech"), Some("Limited"));
    assert!(runner.stats.iter().all(|s| s.name != "Role"));
}

/// Opt-in live check:
/// `cargo test -p marathon --test marathonguide -- --ignored`.
/// Marathon Guide has no Cloudflare gate, so this needs no `FlareSolverr`.
#[tokio::test]
#[ignore = "hits the live Marathon Guide site"]
async fn live_weapon_and_runner_parse_non_empty() {
    let client = reqwest::Client::new();
    let guide = marathon::transport::MarathonGuide::new(client);

    let page = guide.weapons().await.expect("live weapons card");
    let weapon = parse::marathonguide_html_to_weapon("m77-assault-rifle", &page)
        .expect("live m77");
    assert_eq!(weapon.name, "M77 Assault Rifle");
    assert!(weapon.range.is_some(), "live weapon should carry stats");

    let rook = guide.runner("rook").await.expect("live runner");
    let runner = parse::marathonguide_html_to_runner("rook", &rook);
    assert_eq!(runner.name, "Rook");
    assert!(!runner.abilities.is_empty(), "live runner should carry abilities");
}
