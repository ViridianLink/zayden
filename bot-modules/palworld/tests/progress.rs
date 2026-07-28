//! `progress::compute` - milestones from a decoded save.
//!
//! The unit cases build `PlayerRecord`s by hand so each milestone's arithmetic
//! is pinned without a save file. The guild-isolation case runs against the
//! committed multi-guild `progressed-world` fixture, because that is the only
//! place the pooled-vs-personal distinction can actually be observed.

use std::collections::{BTreeMap, BTreeSet};

use palworld::model::{Gender, OwnedPal, PlayerRoster};
use palworld::progress::{CAPTURE_TIERS, Region, catalogue, compute, world_to_map};
use palworld::save::load_world;
use palworld::save::player::{PlayerRecord, load_player};

pub mod common;
use common::progressed_world as world_dir;

fn roster(name: &str) -> PlayerRoster {
    PlayerRoster {
        uid: "00000000000000000000000000000000".to_string(),
        name: name.to_string(),
        level: 42,
        exp: 1000,
        ..PlayerRoster::default()
    }
}

fn pal(species: &str, alpha: bool, lucky: bool) -> OwnedPal {
    OwnedPal {
        species: species.to_string(),
        gender: Gender::Female,
        is_alpha: alpha,
        is_lucky: lucky,
        ..OwnedPal::default()
    }
}

fn starred(species: &str, stars: u8) -> OwnedPal {
    OwnedPal { species: species.to_string(), stars, ..OwnedPal::default() }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn world_to_map_matches_the_reference_transform() {
    // The transform's own fixed point: the world origin reads as (-344, 270)
    // in game. Both references agree on that vertical sign - palworld-save-pal
    // renders `worldToMap(x, y).y * -1` in every tooltip and in the map-corner
    // readout, and PalworldSaveTools' calibration label computes the same
    // `(worldX + 123888) / 459` directly.
    assert_eq!(world_to_map(0.0, 0.0), (-344, 270));
    // Non-finite coordinates must not panic or produce nonsense.
    assert_eq!(world_to_map(f64::NAN, f64::INFINITY), (0, 0));
}

/// The World Tree is a separate map, so its collectibles must never be counted
/// into a Palpagos total. Pooling them is what produced "49/174 fast travel
/// points" with World Tree statues listed at coordinates that are open water on
/// the main map.
#[test]
fn the_two_maps_are_counted_separately() {
    let cat = catalogue();

    for (palpagos, tree) in [
        ("fast-travel", "tree-fast-travel"),
        ("towers", "tree-towers"),
        ("bosses", "tree-bosses"),
        ("bounties", "tree-bounties"),
        ("effigies", "tree-effigies"),
        ("relics", "tree-relics"),
    ] {
        let progress =
            compute(Some(&PlayerRecord::default()), &roster("Fresh"), cat);
        let main = progress.milestone(palpagos).expect(palpagos);
        let world_tree = progress.milestone(tree).expect(tree);

        assert_eq!(main.map, Some(Region::Palpagos), "{palpagos}");
        assert_eq!(world_tree.map, Some(Region::WorldTree), "{tree}");
        assert_eq!(main.category, world_tree.category, "{palpagos} vs {tree}");
        assert!(world_tree.total.is_some_and(|t| t > 0), "{tree} has entries");
    }

    // The catalogue splits exactly, with nothing lost between the two maps.
    assert_eq!(cat.fast_travel_on(Region::Palpagos).count(), 157);
    assert_eq!(cat.fast_travel_on(Region::WorldTree).count(), 17);
    assert_eq!(
        cat.fast_travel_on(Region::Palpagos).count()
            + cat.fast_travel_on(Region::WorldTree).count(),
        cat.fast_travel.len()
    );
    assert_eq!(cat.bosses_on(Region::WorldTree).count(), 7);
    assert_eq!(cat.towers_on(Region::Palpagos).count(), 9);
    assert_eq!(cat.towers_on(Region::WorldTree).count(), 4);
    assert_eq!(cat.effigies_on(Region::WorldTree).count(), 15);
}

/// Unlocking a World Tree statue must move the World Tree line and leave the
/// Palpagos line where it was.
#[test]
fn a_world_tree_unlock_does_not_move_the_palpagos_line() {
    let cat = catalogue();
    let statue = cat.fast_travel_on(Region::WorldTree).next().expect("a statue");
    let record = PlayerRecord {
        fast_travel: set(&[statue.id.as_str()]),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Climber"), cat);
    assert_eq!(progress.milestone("fast-travel").expect("m").have, 0);
    assert_eq!(progress.milestone("tree-fast-travel").expect("m").have, 1);
}

/// `RelicObtainForInstanceFlagByType` holds instance ids, so a relic picked up
/// on one map must not fill a slot on the other. Stamina Reduction relics exist
/// only on the World Tree, which makes them the sharpest case.
#[test]
fn relics_are_attributed_to_the_map_they_sit_on() {
    let cat = catalogue();
    let tree_relics: BTreeSet<String> = cat
        .relics_on(Region::WorldTree)
        .filter(|r| r.kind == "stamina_reduction")
        .map(|r| r.id.clone())
        .collect();
    assert_eq!(tree_relics.len(), 30, "every stamina relic is on the World Tree");

    let record = PlayerRecord {
        relics_by_type: std::iter::once((
            "stamina_reduction".to_string(),
            tree_relics,
        ))
        .collect(),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Collector"), cat);
    assert_eq!(progress.milestone("relics").expect("m").have, 0);
    assert_eq!(progress.milestone("tree-relics").expect("m").have, 30);
}

/// `UnlockedWorldMapFlags` names the maps the player has actually reached, so a
/// map they have never opened is reported as locked rather than as a wall of
/// empty bars they are expected to chase.
#[test]
fn maps_the_player_has_not_reached_are_reported_as_locked() {
    let cat = catalogue();

    let main_only =
        PlayerRecord { world_maps: set(&["MainMap"]), ..PlayerRecord::default() };
    let progress = compute(Some(&main_only), &roster("Newcomer"), cat);
    assert!(progress.is_unlocked(Region::Palpagos));
    assert!(!progress.is_unlocked(Region::WorldTree));

    let both = PlayerRecord {
        world_maps: set(&["MainMap", "Tree"]),
        ..PlayerRecord::default()
    };
    let progress = compute(Some(&both), &roster("Veteran"), cat);
    assert_eq!(progress.unlocked_maps, vec![Region::Palpagos, Region::WorldTree]);
}

/// The headline must not shift just because a category is reported per map.
/// Averaging the halves would let the World Tree's 17 statues count for as much
/// as Palpagos' 157.
#[test]
fn the_headline_weighs_a_category_by_its_size_not_its_map_count() {
    let cat = catalogue();
    // Every Palpagos statue, no World Tree statue: 157 of the 174 that exist.
    let record = PlayerRecord {
        fast_travel: cat
            .fast_travel_on(Region::Palpagos)
            .map(|p| p.id.clone())
            .collect(),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Islander"), cat);
    assert_eq!(progress.milestone("fast-travel").expect("m").have, 157);
    assert_eq!(progress.milestone("tree-fast-travel").expect("m").have, 0);

    // One weighted category is at 157/174; the rest are at zero. Averaging the
    // two halves instead would score the category at 50%.
    let categories: BTreeSet<&str> = progress
        .milestones
        .iter()
        .filter(|m| m.weighted && m.total.is_some())
        .map(|m| m.category)
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a small constant")]
    let expected = (157.0 / 174.0) / categories.len() as f64;
    assert!(
        (progress.overall() - expected).abs() < 1e-9,
        "overall was {}, expected {expected}",
        progress.overall()
    );
}

#[test]
fn an_empty_record_reports_zero_without_dividing_by_zero() {
    let cat = catalogue();
    let progress = compute(Some(&PlayerRecord::default()), &roster("Fresh"), cat);

    assert!(progress.has_player_save);
    assert_eq!(progress.level, 42);
    assert!(!progress.game_cleared);
    assert!((progress.overall() - 0.0).abs() < f64::EPSILON);

    let palpagos = cat.fast_travel_on(Region::Palpagos).count();
    let fast_travel = progress.milestone("fast-travel").expect("milestone");
    assert_eq!(fast_travel.have, 0);
    assert_eq!(fast_travel.total, Some(palpagos));
    assert_eq!(fast_travel.missing.len(), palpagos);
    assert!(!fast_travel.is_complete());
}

#[test]
fn a_complete_record_reads_as_one_hundred_percent() {
    let cat = catalogue();
    let record = PlayerRecord {
        fast_travel: cat.fast_travel.iter().map(|p| p.id.clone()).collect(),
        towers_defeated: cat.towers.iter().map(|t| t.flag.clone()).collect(),
        bosses_defeated: cat.bosses.iter().map(|b| b.spawner.clone()).collect(),
        effigies: cat.effigies().map(|r| r.id.clone()).collect(),
        relics_by_type: cat
            .relic_types
            .iter()
            .map(|t| {
                let ids = cat
                    .relics
                    .iter()
                    .filter(|r| r.kind == t.key)
                    .map(|r| r.id.clone())
                    .collect();
                (t.key.clone(), ids)
            })
            .collect(),
        paldeck_seen: cat.pals.iter().map(|p| p.id.clone()).collect(),
        pal_captures: cat.pals.iter().map(|p| (p.id.clone(), 10)).collect(),
        technologies: cat.technologies.iter().map(|t| t.id.clone()).collect(),
        quests_completed: cat.missions.iter().map(|m| m.id.clone()).collect(),
        areas_found: cat.areas.iter().cloned().collect(),
        arena_ranks: ["Bronze", "Silver", "Gold", "Platinum", "Diamond"]
            .iter()
            .map(|rank| ((*rank).to_string(), 1))
            .collect(),
        game_cleared: true,
        ..PlayerRecord::default()
    };

    // `condensed` is roster-derived, so a 100% record still needs one 4-star
    // Pal of every species to sit alongside it.
    let mut owner = roster("Done");
    owner.personal_pals = cat
        .pals
        .iter()
        .map(|p| OwnedPal { species: p.id.clone(), stars: 4, ..OwnedPal::default() })
        .collect();

    let progress = compute(Some(&record), &owner, cat);
    assert!(progress.game_cleared);

    for milestone in progress.milestones.iter().filter(|m| m.total.is_some()) {
        assert!(
            milestone.is_complete(),
            "{} is {}/{:?}",
            milestone.label,
            milestone.have,
            milestone.total
        );
        assert!(milestone.missing.is_empty(), "{} lists leftovers", milestone.label);
    }

    assert!(
        (progress.overall() - 1.0).abs() < 1e-9,
        "overall was {}",
        progress.overall()
    );
}

/// The 5x tier counts species whose total captures reach the threshold, and is
/// deliberately excluded from the overall percentage. Only the two ends of the
/// game's 1/3/5/7/10 ladder are tracked.
#[test]
fn the_five_times_tier_counts_species_at_its_threshold() {
    let cat = catalogue();
    // Internal ids, which is what the save writes: Lamball is `Sheepball`,
    // Chikipi is `ChickenPal`, Foxparks is `Kitsunebi`, Melpaca is `Alpaca`.
    let counts =
        [("Sheepball", 12), ("ChickenPal", 6), ("Kitsunebi", 3), ("Alpaca", 1)];
    let record = PlayerRecord {
        pal_captures: counts
            .iter()
            .map(|(id, n)| ((*id).to_string(), *n))
            .collect::<BTreeMap<String, i64>>(),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Catcher"), cat);

    assert_eq!(progress.milestone("captures").expect("m").have, 4);
    assert_eq!(progress.milestone("captures-5x").expect("m").have, 2);
    assert_eq!(CAPTURE_TIERS, [1, 5]);

    for key in ["captures-3x", "captures-7x", "captures-10x"] {
        assert!(progress.milestone(key).is_none(), "{key} is no longer tracked");
    }

    assert!(
        !progress.milestone("captures-5x").expect("m").weighted,
        "the bonus tier must not feed the overall percentage"
    );
    assert!(progress.milestone("captures").expect("m").weighted);
}

/// Condensing is per-species and reads the best copy the player owns, so a spare
/// un-condensed duplicate never masks a finished one.
#[test]
fn condensed_counts_the_best_copy_of_each_species() {
    let cat = catalogue();
    let mut owner = roster("Condenser");
    owner.personal_pals = vec![
        starred("Sheepball", 4),
        starred("Sheepball", 0), // a spare that must not drag the species down
        starred("ChickenPal", 3),
        starred("Kitsunebi", 4),
    ];
    // The guild shares a fully condensed Pal this player did not earn.
    owner.pals = owner.personal_pals.clone();
    owner.pals.push(starred("Alpaca", 4));

    let progress = compute(None, &owner, cat);
    let condensed = progress.milestone("condensed").expect("m");

    assert_eq!(condensed.have, 2, "only Sheepball and Kitsunebi are at 4 stars");
    assert_eq!(condensed.total, Some(cat.pals.len()));
    assert!(!condensed.weighted, "the condense grind is not weighted");

    let note = condensed.note.clone().expect("note");
    assert!(note.contains("2 Pals at 4★"), "note was {note}");
    assert!(note.contains("1 partly condensed"), "note was {note}");

    // A single fully condensed Pal reads as "1 Pal", not "1 Pals".
    let mut solo = roster("Starter");
    solo.personal_pals = vec![starred("Sheepball", 4)];
    let note = compute(None, &solo, cat)
        .milestone("condensed")
        .and_then(|m| m.note.clone())
        .expect("note");
    assert!(note.contains("1 Pal at 4★"), "note was {note}");

    // The drill-down covers every species bar the two that are done, saying how
    // far along a partly condensed one is and marking those never owned.
    let missing: Vec<&str> =
        condensed.missing.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(missing.len(), cat.pals.len() - 2, "the finished species drop out");
    assert!(missing.iter().any(|m| m.contains("(3★/4★)")), "{missing:?}");
    assert!(missing.iter().any(|m| m.contains("(not owned)")), "{missing:?}");
}

/// Condensing is progress only for the player who did it - a guildmate's
/// 4-star base worker is pooled into `pals` but must never be counted.
#[test]
fn guild_pooled_pals_do_not_count_as_condensed() {
    let cat = catalogue();
    let mut owner = roster("Freeloader");
    owner.personal_pals = vec![starred("Sheepball", 1)];
    owner.pals = vec![
        starred("Sheepball", 1),
        starred("ChickenPal", 4),
        starred("Alpaca", 4),
    ];

    let condensed =
        compute(None, &owner, cat).milestone("condensed").cloned().expect("m");

    assert_eq!(condensed.have, 0, "pooled 4-star Pals are not this player's");
}

/// `PalCaptureCount` also carries humans and raid monsters. They are not Paldeck
/// entries and must not count toward a completion ratio capped at 303.
#[test]
fn non_pal_capture_keys_are_ignored() {
    let cat = catalogue();
    let record = PlayerRecord {
        pal_captures: [("Sheepball", 1), ("Human", 40), ("YakushimaMonster001", 9)]
            .iter()
            .map(|(id, n)| ((*id).to_string(), *n))
            .collect(),
        paldeck_seen: set(&["Sheepball", "Human"]),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Hunter"), cat);
    assert_eq!(progress.milestone("captures").expect("m").have, 1);
    assert_eq!(progress.milestone("paldeck").expect("m").have, 1);
}

/// Bounty targets are the subset of bosses that drop a token, so beating a
/// non-bounty boss must move `bosses` without moving `bounties`.
#[test]
fn bounties_track_only_token_dropping_bosses() {
    let cat = catalogue();
    let plain = cat
        .bosses_on(Region::Palpagos)
        .find(|b| !b.bounty)
        .expect("a non-bounty Palpagos boss");
    let record = PlayerRecord {
        bosses_defeated: set(&[plain.spawner.as_str()]),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Slayer"), cat);
    assert_eq!(progress.milestone("bosses").expect("m").have, 1);
    assert_eq!(progress.milestone("bounties").expect("m").have, 0);
}

/// Spawner ids are compared case-insensitively, matching how the save writes
/// them (`Boss_Anubis` vs the catalogue's `BOSS_Anubis`).
#[test]
fn boss_spawners_match_case_insensitively() {
    let cat = catalogue();
    let boss = cat.bosses_on(Region::Palpagos).next().expect("a boss");
    let record = PlayerRecord {
        bosses_defeated: set(&[boss.spawner.to_uppercase().as_str()]),
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Slayer"), cat);
    assert_eq!(progress.milestone("bosses").expect("m").have, 1);
}

/// Uncapped counters report a number and stay out of the percentage.
#[test]
fn counters_without_a_catalogue_are_unranked() {
    let cat = catalogue();
    let record = PlayerRecord {
        notes: set(&["Day1", "Day2", "ForestBoss1"]),
        normal_dungeons_cleared: 12,
        fixed_dungeons_cleared: 5,
        camps_conquered: 4,
        oilrigs_cleared: 2,
        ..PlayerRecord::default()
    };

    let progress = compute(Some(&record), &roster("Explorer"), cat);

    let notes = progress.milestone("notes").expect("m");
    assert_eq!(notes.have, 3);
    assert_eq!(notes.total, None);
    assert_eq!(notes.fraction(), None);
    assert!(!notes.weighted);

    assert_eq!(progress.milestone("dungeons").expect("m").have, 17);
    assert_eq!(progress.milestone("deeds").expect("m").have, 6);
}

/// The arena's denominator comes from a hard-coded rank list, so it has to cope
/// with a build that adds a sixth rank: `have` may never exceed `total`.
#[test]
fn arena_denominator_grows_with_unknown_ranks() {
    let cat = catalogue();

    let known = PlayerRecord {
        arena_ranks: [("Bronze", 3), ("Silver", 1)]
            .iter()
            .map(|(r, n)| ((*r).to_string(), *n))
            .collect(),
        ..PlayerRecord::default()
    };
    let arena = compute(Some(&known), &roster("Fighter"), cat);
    let arena = arena.milestone("arena").expect("m");
    assert_eq!((arena.have, arena.total), (2, Some(5)));
    assert_eq!(arena.missing.len(), 3);
    // A side mode with five entries must not sway the headline percentage.
    assert!(!arena.weighted);

    let future = PlayerRecord {
        arena_ranks: ["Bronze", "Silver", "Gold", "Platinum", "Diamond", "Mythic"]
            .iter()
            .map(|r| ((*r).to_string(), 1))
            .collect(),
        ..PlayerRecord::default()
    };
    let arena = compute(Some(&future), &roster("Champion"), cat);
    let arena = arena.milestone("arena").expect("m");
    assert_eq!((arena.have, arena.total), (6, Some(6)));
    assert!(arena.is_complete(), "an unrecognised rank still counts as cleared");

    // A rank listed with zero clears is entered, not beaten.
    let entered = PlayerRecord {
        arena_ranks: std::iter::once(("Bronze".to_string(), 0)).collect(),
        ..PlayerRecord::default()
    };
    let arena = compute(Some(&entered), &roster("Rookie"), cat);
    assert_eq!(arena.milestone("arena").expect("m").have, 0);
}

/// Newer builds dropped `FoundTreasureCount` in favour of a per-instance map. The
/// deeds line has to read whichever the save carries.
#[test]
fn treasures_read_from_either_the_scalar_or_the_map() {
    let cat = catalogue();

    let legacy = PlayerRecord { treasures_found: 4, ..PlayerRecord::default() };
    assert_eq!(legacy.treasures(), 4);
    assert_eq!(
        compute(Some(&legacy), &roster("Old"), cat)
            .milestone("deeds")
            .expect("m")
            .have,
        4
    );

    let current = PlayerRecord {
        treasure_points: set(&["A1", "B2", "C3"]),
        ..PlayerRecord::default()
    };
    assert_eq!(current.treasures(), 3);
    assert_eq!(
        compute(Some(&current), &roster("New"), cat)
            .milestone("deeds")
            .expect("m")
            .have,
        3
    );

    // A save carried across the update keeps both; they describe the same digs.
    let both = PlayerRecord {
        treasures_found: 3,
        treasure_points: set(&["A1", "B2", "C3"]),
        ..PlayerRecord::default()
    };
    assert_eq!(both.treasures(), 3, "counted once, not six times");
}

/// Without the player's own save only the roster-derived milestones exist.
#[test]
fn a_missing_player_save_still_reports_the_roster() {
    let cat = catalogue();
    let mut player = roster("Unlinked");
    player.personal_pals = vec![pal("Sheepball", false, false)];

    let progress = compute(None, &player, cat);
    assert!(!progress.has_player_save);

    let keys: Vec<&str> = progress.milestones.iter().map(|m| m.key).collect();
    assert_eq!(keys, ["condensed", "collection"]);
    assert_eq!(progress.milestone("collection").expect("m").have, 1);
    assert_eq!(progress.milestone("condensed").expect("m").have, 0);
}

/// The collection milestone reads `personal_pals`. Guild-pooled base workers
/// live in `pals` and must not appear in anyone's progress.
#[test]
fn collection_ignores_guild_pooled_pals() {
    let cat = catalogue();
    let mut player = roster("Guilded");
    player.personal_pals = vec![
        pal("Sheepball", false, false),
        pal("BOSS_ChickenPal", true, false),
        pal("Kitsunebi", false, true),
    ];
    // Ten more Pals the guild shares - visible to breeding, invisible to
    // progression.
    player.pals = player.personal_pals.clone();
    player.pals.extend((0..10).map(|_| pal("Alpaca", false, false)));

    let collection =
        compute(None, &player, cat).milestone("collection").cloned().expect("m");

    assert_eq!(collection.have, 3, "only personally-owned Pals count");
    let note = collection.note.expect("note");
    assert!(note.contains("3 species"), "note was {note}");
    assert!(note.contains("1 alpha"), "note was {note}");
    assert!(note.contains("1 lucky"), "note was {note}");
}

/// The regression that stops guild Pals leaking into progression, checked
/// against a real world where guilds actually pool hundreds of base Pals.
#[test]
fn guild_pools_never_reach_a_players_progress() {
    let dir = world_dir();
    let cat = catalogue();
    let world = load_world(&dir).expect("load_world");
    let mut checked = 0usize;

    for player in &world.players {
        // Pooling only shows up for guild members, who see more than they own.
        if player.pals.len() <= player.personal_pals.len() {
            continue;
        }
        checked += 1;

        let record = load_player(&dir, &player.uid).expect("decode");
        let progress = compute(record.as_ref(), player, cat);
        let collection = progress.milestone("collection").expect("m");

        assert_eq!(
            collection.have,
            player.personal_pals.len(),
            "{}'s collection counts their own Pals",
            player.name
        );
        assert!(
            collection.have < player.pals.len(),
            "{}'s progress is strictly below their pooled roster",
            player.name
        );

        // Paldeck and captures come from the player's own RecordData, so they
        // cannot exceed what that record holds however large the guild pool is.
        if let Some(record) = record.as_ref() {
            let paldeck = progress.milestone("paldeck").expect("m");
            assert!(
                paldeck.have <= record.paldeck_seen.len(),
                "{}'s paldeck exceeds their own record",
                player.name
            );
            let captures = progress.milestone("captures").expect("m");
            assert!(
                captures.have <= record.pal_captures.len(),
                "{}'s captures exceed their own record",
                player.name
            );
        }
    }

    assert!(checked > 0, "the real world has at least one pooling guild member");
}

/// End-to-end shape check on the real world: percentages stay in range and a
/// finished character outranks a new one.
#[test]
fn real_world_progress_is_ordered_and_in_range() {
    let dir = world_dir();
    let cat = catalogue();
    let world = load_world(&dir).expect("load_world");

    let mut scored: Vec<(String, f64, bool)> = Vec::new();
    for player in &world.players {
        let record = load_player(&dir, &player.uid).expect("decode");
        if record.is_none() {
            continue;
        }
        let progress = compute(record.as_ref(), player, cat);
        let overall = progress.overall();
        assert!(
            (0.0..=1.0).contains(&overall),
            "{} scored {overall}",
            progress.player
        );
        for milestone in &progress.milestones {
            if let (Some(total), Some(fraction)) =
                (milestone.total, milestone.fraction())
            {
                assert!(milestone.have <= total, "{} overflowed", milestone.label);
                assert!((0.0..=1.0).contains(&fraction));
            }
        }
        scored.push((progress.player.clone(), overall, progress.game_cleared));
    }

    assert!(scored.len() >= 2, "the real world has several players");

    // Anyone who has cleared the story is ahead of everyone who has not.
    let worst_cleared = scored
        .iter()
        .filter(|(_, _, cleared)| *cleared)
        .map(|(_, score, _)| *score)
        .fold(f64::INFINITY, f64::min);
    if worst_cleared.is_finite() {
        for (name, score, cleared) in &scored {
            assert!(
                *cleared || *score <= worst_cleared,
                "{name} outscores a story-cleared character"
            );
        }
    }
}
