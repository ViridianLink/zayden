//! `save::player` - decoding `Players/<uid>.sav`.
//!
//! Three fixtures, each pinning something the others cannot. `steam-world1` is a
//! near-fresh two-player world whose `RecordData` carries only the three or four
//! keys the game has had reason to write - the "absent key reads as empty" case.
//! `progressed-world` is a finished 8-player Feybreak-era save that exercises
//! every field. `storage-world` is the current build, and the only one carrying
//! the keys that build added.

use std::path::PathBuf;

use palworld::save::player::{load_player, parse_player_uid, unknown_record_keys};
use palworld::save::{load_world, uid_to_filename};

pub mod common;
use common::{progressed_world, storage_world};

/// `43797F87…` on disk; `877F7943…` in the byte order this crate stores.
const FIXTURE_UID_A: &str = "877F7943000000000000000000000000";
/// `8C2F1930…` on disk - the fixture's more-played character.
const FIXTURE_UID_B: &str = "30192F8C000000000000000000000000";

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/steam-world1")
}

#[test]
fn uid_to_filename_round_trips() {
    let on_disk = "43797F87000000000000000000000000";
    assert_eq!(uid_to_filename(FIXTURE_UID_A).as_deref(), Some(on_disk));
    // Unreal writes the first three GUID groups little-endian, so the same
    // reversal maps back - `load_player` relies on that to find the file.
    assert_eq!(uid_to_filename(on_disk).as_deref(), Some(FIXTURE_UID_A));
    assert_eq!(uid_to_filename("not-a-guid"), None);
}

#[test]
fn parses_fixture_player_saves() {
    let dir = fixture_dir();

    let a = load_player(&dir, FIXTURE_UID_A).expect("decode").expect("present");
    let b = load_player(&dir, FIXTURE_UID_B).expect("decode").expect("present");

    assert_eq!(a.uid, FIXTURE_UID_A);

    // Both characters unlocked the same single fast-travel point and no more.
    let point = "41E36D9A4B2BA79A3AD1B7B83B16F77D";
    assert_eq!(a.fast_travel.len(), 1);
    assert!(a.fast_travel.contains(point));
    assert_eq!(b.fast_travel, a.fast_travel);

    // B has wandered further: three discovered areas to A's one.
    assert_eq!(a.areas_found.len(), 1);
    assert!(a.areas_found.contains("PvPIsland_002"));
    assert_eq!(b.areas_found.len(), 3);
    assert!(b.areas_found.contains("Sakurajima_Sakura"));

    // B has spoken to one NPC; A has no `NPCTalkCountMap` at all.
    assert_eq!(b.npcs_talked.values().copied().sum::<i64>(), 2);
    assert!(a.npcs_talked.is_empty());

    // The EXP-bonus tiers the two have earned differ, and are read as scalars.
    assert_eq!(a.exp_bonus_tiers.get("Area"), Some(&1));
    assert_eq!(b.exp_bonus_tiers.get("Area"), Some(&3));

    assert_eq!(a.technology_points, 7);
}

/// A near-fresh save carries none of the collection keys. Every one of them must
/// read as empty rather than erroring or being skipped.
#[test]
fn absent_record_keys_read_as_empty() {
    let a = load_player(&fixture_dir(), FIXTURE_UID_A)
        .expect("decode")
        .expect("present");

    assert!(a.paldeck_seen.is_empty());
    assert!(a.pal_captures.is_empty());
    assert!(a.effigies.is_empty());
    assert!(a.bosses_defeated.is_empty());
    assert!(a.towers_defeated.is_empty());
    assert!(a.notes.is_empty());
    assert!(a.relics_by_type.is_empty());
    assert_eq!(a.tribe_captures, 0);
    assert_eq!(a.boss_technology_points, 0);
    assert!(!a.game_cleared);
}

#[test]
fn missing_player_save_is_none_not_an_error() {
    let absent = "FFFFFFFF000000000000000000000000";
    assert_eq!(load_player(&fixture_dir(), absent).expect("decode"), None);
    // A uid that isn't a GUID can't name a file, and is likewise not an error.
    assert_eq!(load_player(&fixture_dir(), "bogus").expect("decode"), None);
}

#[test]
fn player_uid_is_read_from_the_save_not_the_filename() {
    let path =
        fixture_dir().join("Players").join("8C2F1930000000000000000000000000.sav");
    let raw = std::fs::read(path).expect("fixture present");
    assert_eq!(parse_player_uid(&raw).expect("uid"), FIXTURE_UID_B);
}

/// The two save kinds are told apart by their GVAS root, not their file name -
/// an upload named `Level.sav` that is really a player save (or vice versa) has
/// to be rejected, or it would be stored under the wrong one.
#[test]
fn world_and_player_saves_are_not_interchangeable() {
    let level = std::fs::read(fixture_dir().join("Level.sav")).expect("fixture");
    let player = std::fs::read(
        fixture_dir().join("Players").join("8C2F1930000000000000000000000000.sav"),
    )
    .expect("fixture");

    assert!(parse_player_uid(&level).is_err(), "Level.sav has no SaveData");
    assert!(palworld::save::validate_level(&level).is_ok(), "Level.sav is a world");

    assert!(parse_player_uid(&player).is_ok(), "a player save declares its uid");
    assert!(
        palworld::save::validate_level(&player).is_err(),
        "a player save is valid GVAS but is not a world save",
    );
}

/// Every `RecordData` key the game writes should be one this module reads. A new
/// key here means an update added progression we are silently dropping.
///
/// Both worlds are checked: they are different game builds, and the keys they
/// carry only partly overlap.
#[test]
fn real_saves_carry_no_unknown_record_keys() {
    let mut unknown: Vec<String> = Vec::new();

    for dir in [progressed_world(), storage_world()] {
        let world = load_world(&dir).expect("load_world");
        for player in &world.players {
            let Some(path) = palworld::save::player_save_path(&dir, &player.uid)
            else {
                continue;
            };
            let Ok(raw) = std::fs::read(&path) else { continue };
            unknown.extend(unknown_record_keys(&raw).expect("decode"));
        }
    }
    unknown.sort_unstable();
    unknown.dedup();

    assert!(
        unknown.is_empty(),
        "unhandled RecordData key(s): {unknown:?} - add them to \
         save::player::PlayerRecord and to the KNOWN list",
    );
}

/// The struct-keyed `FoundTreasureMapPointMap` needs a GVAS hint to decode at
/// all. Without it the player save is not merely missing a field - the whole
/// parse fails, and `/palworld progress` reports an unreadable world.
#[test]
fn struct_keyed_treasure_map_decodes() {
    let dir = storage_world();
    // Oscar Six: `B0726C28…` on disk.
    let oscar = "286C72B0000000000000000000000000";

    let record = load_player(&dir, oscar).expect("decode").expect("present");
    assert_eq!(record.treasure_points.len(), 1, "treasure instance ids decoded");

    // This build dropped the `FoundTreasureCount` scalar, so the map is the only
    // place the count survives.
    assert_eq!(record.treasures_found, 0);
    assert_eq!(record.treasures(), 1, "the map stands in for the missing scalar");
}

/// `ArenaSoloClearCount` is absent on every player of a pre-arena build, and a
/// per-rank count on a current one.
#[test]
fn arena_ranks_are_read_per_rank() {
    let dir = storage_world();
    // KingJosh has cleared all five ranks; Kitty has never entered the arena.
    let kingjosh = "5CF598C9000000000000000000000000";
    let kitty = "D0C08D37000000000000000000000000";

    let cleared = load_player(&dir, kingjosh).expect("decode").expect("present");
    assert_eq!(cleared.arena_ranks.len(), 5);
    assert_eq!(cleared.arena_ranks.get("Platinum"), Some(&1));

    let none = load_player(&dir, kitty).expect("decode").expect("present");
    assert!(none.arena_ranks.is_empty());

    // Mutations are new in this build too, and are a plain scalar.
    assert_eq!(none.mutations, 15);
}

/// The progressed world exercises what the fixture cannot: populated collections
/// across every field the progress command reads.
#[test]
fn decodes_a_progressed_world() {
    let dir = progressed_world();
    let world = load_world(&dir).expect("load_world");

    let records: Vec<_> = world
        .players
        .iter()
        .filter_map(|p| load_player(&dir, &p.uid).expect("decode"))
        .collect();
    assert!(!records.is_empty(), "a populated world has player saves");

    let best = records
        .iter()
        .max_by_key(|r| r.paldeck_seen.len())
        .expect("at least one record");

    assert!(!best.paldeck_seen.is_empty(), "paldeck decoded");
    assert!(!best.pal_captures.is_empty(), "captures decoded");
    assert!(!best.fast_travel.is_empty(), "fast travel decoded");
    assert!(!best.technologies.is_empty(), "technologies decoded");
    assert!(!best.quests_completed.is_empty(), "quest array decoded");

    // Captures are counts, not flags: at least one species caught more than once.
    assert!(
        best.pal_captures.values().any(|&n| n > 1),
        "PalCaptureCount carries per-species counts"
    );

    // Someone in a finished world has beaten a tower and collected effigies.
    assert!(
        records.iter().any(|r| !r.towers_defeated.is_empty()),
        "tower flags decoded"
    );
    assert!(records.iter().any(|r| !r.effigies.is_empty()), "effigy flags decoded");
    assert!(
        records.iter().any(|r| !r.relics_by_type.is_empty()),
        "relic counts decoded, keyed by our snake_case type keys"
    );
}
