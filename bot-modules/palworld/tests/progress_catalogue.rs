//! `progress::catalogue` - the embedded reference data.
//!
//! The counts below are the denominators every percentage `/palworld progress`
//! reports is divided by, so a bad re-sync of `data/` has to fail here rather
//! than silently skew the numbers. See `data/README.md` for provenance.

use palworld::progress::{Region, catalogue};

#[test]
fn every_catalogue_parses_with_the_expected_count() {
    let cat = catalogue();

    assert_eq!(cat.fast_travel.len(), 174, "fast travel points");
    assert_eq!(cat.bosses.len(), 125, "boss spawners");
    assert_eq!(cat.relics.len(), 406, "relic pickups");
    assert_eq!(cat.relic_types.len(), 13, "relic types");
    assert_eq!(cat.technologies.len(), 591, "technologies");
    assert_eq!(cat.missions.len(), 120, "missions");
    assert_eq!(cat.areas.len(), 125, "map areas");
    assert_eq!(cat.towers.len(), 13, "tower battles");
    assert_eq!(cat.pals.len(), 303, "paldeck entries");
}

#[test]
fn derived_subsets_match_their_documented_sizes() {
    let cat = catalogue();

    // The Lifmunk Effigies are the `capture_power` relics - the save format
    // calls them relics, and the flat `RelicObtainForInstanceFlag` tracks these.
    assert_eq!(cat.effigies().count(), 154, "effigies");

    // `hidden` quests are internal triggers and must not deflate the ratio.
    assert_eq!(cat.tracked_missions().count(), 117, "player-facing missions");

    assert_eq!(cat.bosses.iter().filter(|b| b.alpha).count(), 89, "alphas");
    assert_eq!(
        cat.bosses.iter().filter(|b| b.bounty).count(),
        89,
        "bounty-token bosses"
    );
    assert_eq!(
        cat.technologies.iter().filter(|t| t.boss).count(),
        51,
        "boss technologies"
    );
    assert_eq!(
        cat.fast_travel.iter().filter(|p| p.kind == "map_point").count(),
        22,
        "map-unlock points"
    );
}

/// Every located entry carries the map it sits on, and the two maps partition
/// the catalogue - nothing may be counted twice or fall between them.
#[test]
fn every_located_catalogue_splits_cleanly_between_the_two_maps() {
    let cat = catalogue();

    assert_eq!(cat.fast_travel_on(Region::Palpagos).count(), 157);
    assert_eq!(cat.fast_travel_on(Region::WorldTree).count(), 17);
    assert_eq!(cat.bosses_on(Region::Palpagos).count(), 118);
    assert_eq!(cat.bosses_on(Region::WorldTree).count(), 7);
    assert_eq!(cat.relics_on(Region::Palpagos).count(), 359);
    assert_eq!(cat.relics_on(Region::WorldTree).count(), 47);
    assert_eq!(cat.effigies_on(Region::Palpagos).count(), 139);
    assert_eq!(cat.effigies_on(Region::WorldTree).count(), 15);
    assert_eq!(cat.towers_on(Region::Palpagos).count(), 9);
    assert_eq!(cat.towers_on(Region::WorldTree).count(), 4);

    for map in Region::ALL {
        assert!(
            cat.fast_travel_on(map).count() > 0,
            "{} has fast travel points",
            map.label()
        );
    }
    let split = |palpagos: usize, tree: usize, whole: usize, what: &str| {
        assert_eq!(palpagos + tree, whole, "{what} lost an entry between the maps");
    };
    split(
        cat.fast_travel_on(Region::Palpagos).count(),
        cat.fast_travel_on(Region::WorldTree).count(),
        cat.fast_travel.len(),
        "fast travel",
    );
    split(
        cat.bosses_on(Region::Palpagos).count(),
        cat.bosses_on(Region::WorldTree).count(),
        cat.bosses.len(),
        "bosses",
    );
    split(
        cat.relics_on(Region::Palpagos).count(),
        cat.relics_on(Region::WorldTree).count(),
        cat.relics.len(),
        "relics",
    );
    split(
        cat.effigies_on(Region::Palpagos).count(),
        cat.effigies_on(Region::WorldTree).count(),
        cat.effigies().count(),
        "effigies",
    );
    split(
        cat.towers_on(Region::Palpagos).count(),
        cat.towers_on(Region::WorldTree).count(),
        cat.towers.len(),
        "towers",
    );

    // Map discovery is a Palpagos-only mechanic: the areas list holds
    // `FootOfWorldTree`, which is the approach on the main map, and nothing on
    // the World Tree itself.
    assert!(cat.areas.iter().any(|a| a == "FootOfWorldTree"));
    assert!(
        !cat.areas.iter().any(|a| a.starts_with("WorldTree_")),
        "the World Tree contributes no discovery areas"
    );
}

/// The World Tree's landmarks are on the far side of the world from Palpagos,
/// which is exactly why pooling their totals sent players to open water.
#[test]
fn the_two_maps_occupy_disjoint_world_space() {
    let cat = catalogue();
    let max_palpagos_x = cat
        .fast_travel_on(Region::Palpagos)
        .map(|p| p.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_tree_x = cat
        .fast_travel_on(Region::WorldTree)
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);

    assert!(
        max_palpagos_x < min_tree_x,
        "Palpagos reaches {max_palpagos_x}, the World Tree starts at {min_tree_x}"
    );
}

#[test]
fn ids_are_unique_within_each_catalogue() {
    let cat = catalogue();

    let unique = |mut ids: Vec<&str>| {
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        (ids.len(), total)
    };

    let (uniq, total) =
        unique(cat.fast_travel.iter().map(|p| p.id.as_str()).collect());
    assert_eq!(uniq, total, "duplicate fast-travel id");
    let (uniq, total) =
        unique(cat.bosses.iter().map(|b| b.spawner.as_str()).collect());
    assert_eq!(uniq, total, "duplicate boss spawner");
    let (uniq, total) = unique(cat.relics.iter().map(|r| r.id.as_str()).collect());
    assert_eq!(uniq, total, "duplicate relic id");
    let (uniq, total) = unique(cat.pals.iter().map(|p| p.id.as_str()).collect());
    assert_eq!(uniq, total, "duplicate pal id");
    let (uniq, total) = unique(cat.towers.iter().map(|t| t.flag.as_str()).collect());
    assert_eq!(uniq, total, "duplicate tower flag");
}

/// The save's spelling of a Pal id does not always match the catalogue's, and
/// boss/predator variants carry a prefix. Both must resolve.
#[test]
fn pal_lookup_is_case_and_prefix_insensitive() {
    let cat = catalogue();

    let sheep = cat.pal("Sheepball").expect("Sheepball is a Pal");
    // The real saves write `SheepBall`; upstream records `Sheepball`.
    assert_eq!(cat.pal("SheepBall").map(|p| &p.id), Some(&sheep.id));
    assert_eq!(cat.pal("BOSS_Sheepball").map(|p| &p.id), Some(&sheep.id));
    assert_eq!(cat.pal("PREDATOR_Sheepball").map(|p| &p.id), Some(&sheep.id));
    assert_eq!(cat.pal("  Sheepball  ").map(|p| &p.id), Some(&sheep.id));

    // Humans and raid monsters appear as `PalCaptureCount` keys but are not
    // Paldeck entries, so they must not inflate the capture ratio.
    assert!(cat.pal("Human").is_none(), "humans are not Paldeck entries");
    assert!(cat.pal("YakushimaMonster001").is_none(), "raid monsters excluded");

    // A few ids are Paldeck entries *and* share a base with another entry.
    // Stripping the prefix first would merge the two, so the exact id wins.
    let summon = cat.pal("SUMMON_DarkAlien").expect("a Paldeck entry itself");
    assert_eq!(summon.id, "SUMMON_DarkAlien");
    assert_eq!(cat.pal("DarkAlien").map(|p| p.id.as_str()), Some("DarkAlien"));
}

/// Tower flags are matched by exact key, so the catalogue's spelling has to be
/// the game's: `BOSS_BATTLE_NAME_<boss type>`.
#[test]
fn tower_flags_use_the_games_key_format() {
    let cat = catalogue();
    for tower in &cat.towers {
        assert_eq!(tower.flag, format!("BOSS_BATTLE_NAME_{}", tower.id));
        assert!(!tower.name.is_empty(), "{} has a display name", tower.id);
    }
    assert!(cat.towers.iter().any(|t| t.id == "GrassBoss"));
    assert!(cat.towers.iter().any(|t| t.id == "WorldTreeMiddleBoss3"));
}

#[test]
fn relic_types_cover_every_relic_in_the_catalogue() {
    let cat = catalogue();
    for relic in &cat.relics {
        assert!(
            cat.relic_type(&relic.kind).is_some(),
            "relic {} has unknown type {}",
            relic.id,
            relic.kind
        );
    }
}
