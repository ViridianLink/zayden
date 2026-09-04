//! The point grants that accompany a level change.
//!
//! Two constants drive this: one status point and six technology points per
//! level. The status figure is derived from the fixtures here rather than taken
//! on trust - `status_point_identity_holds_across_fixtures` is the measurement,
//! and it is what justifies granting exactly one point per level elsewhere.

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use palworld::save::decompress::decompress;
use palworld::save::edit::{
    PlayerEdit,
    STATUS_POINTS_PER_LEVEL,
    SaveEdits,
    apply_edits,
};
use palworld::save::edit_player::{
    TECH_POINTS_PER_LEVEL,
    grant_tech_points,
    tech_points,
};
use palworld::save::extract::{custom_struct, field, int_field, struct_fields};
use palworld::save::gvas::{read_gvas, reparse_properties_at};
use palworld::save::uid_to_filename;

pub mod common;

/// The five stats a level-up point can be spent on, as the save names them.
const CORE_STATS: [&str; 5] = ["最大HP", "最大SP", "攻撃力", "所持重量", "作業速度"];

/// `(level, unused_status_points, points_allocated_to_the_five_core_stats)` for
/// every player character in a world.
///
/// Returns empty on any decode failure rather than panicking - the workspace
/// only exempts `#[test]` bodies from `expect_used`. Silence is not a way to
/// pass: every caller either asserts on a named character or on the total
/// count, so a world that stops decoding fails the test.
fn player_points(file: &gvas::GvasFile) -> Vec<(String, i64, i64, i64)> {
    let versions = file.header.get_custom_versions().clone();

    let Some(world) = custom_struct(file.properties.0.get("worldSaveData")) else {
        return Vec::new();
    };
    let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
        world.0.get("CharacterSaveParameterMap").and_then(|v| v.first())
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (_key, val) in &value.0 {
        let Some(fields) = struct_fields(val) else { continue };
        let Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) =
            field(fields, "RawData")
        else {
            continue;
        };
        let Ok(parsed) = reparse_properties_at(bytes, &versions) else { continue };
        let Some(sp) = parsed
            .properties
            .iter()
            .find(|(n, _)| n == "SaveParameter")
            .and_then(|(_, p)| struct_fields(p))
        else {
            continue;
        };
        let is_player = matches!(
            field(sp, "IsPlayer"),
            Some(Property::BoolProperty(b)) if b.value
        );
        if !is_player {
            continue;
        }

        let name = match field(sp, "NickName") {
            Some(Property::StrProperty(s)) => s.value.clone().unwrap_or_default(),
            _ => String::new(),
        };
        let unused = match field(sp, "UnusedStatusPoint") {
            Some(Property::UInt16Property(p)) => i64::from(p.value),
            _ => 0,
        };
        out.push((name, int_field(sp, "Level"), unused, core_allocated(sp)));
    }
    out
}

fn core_allocated(
    sp: &gvas::types::map::HashableIndexMap<String, Vec<Property>>,
) -> i64 {
    let Some(Property::ArrayProperty(ArrayProperty::Structs { structs, .. })) =
        field(sp, "GotStatusPointList")
    else {
        return 0;
    };
    let mut total = 0;
    for s in structs {
        let StructPropertyValue::CustomStruct(f) = s else { continue };
        let name = match field(f, "StatusName") {
            Some(Property::NameProperty(n)) => n.value.clone().unwrap_or_default(),
            _ => String::new(),
        };
        if !CORE_STATS.contains(&name.as_str()) {
            continue;
        }
        if let Some(Property::IntProperty(p)) = field(f, "StatusPoint") {
            total += i64::from(p.value);
        }
    }
    total
}

/// The measurement behind [`STATUS_POINTS_PER_LEVEL`].
///
/// For every legitimately-levelled character, points spent on the five level-up
/// stats plus the unspent pool equal `level - 1`. That is one point per level
/// with none unaccounted for, and it is why the write path grants exactly one.
///
/// `steam-world1` is deliberately excluded: its level-65 character holds 250
/// core points, which no amount of levelling produces. That save is the reason
/// `grant_status_points` adjusts the unspent pool by a delta instead of
/// recomputing the total from the level - a recompute would silently rewrite a
/// manipulated save's allocations, and it would be wrong to assume the identity
/// holds on input we did not create.
#[test]
fn status_point_identity_holds_across_fixtures() {
    let mut checked = 0;
    for file in [
        common::progressed_gvas().expect("decode progressed-world"),
        common::storage_gvas().expect("decode storage-world"),
    ] {
        for (name, level, unused, core) in player_points(file) {
            assert_eq!(
                core + unused,
                level - i64::from(STATUS_POINTS_PER_LEVEL),
                "{name} (level {level}): {core} allocated + {unused} unspent \
                 should account for every level-up point"
            );
            checked += 1;
        }
    }
    // 8 in progressed-world, 3 in storage-world.
    assert_eq!(checked, 11, "fixtures should supply 11 player characters");
}

#[test]
fn raising_a_level_grants_one_status_point_per_level() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let before = common::progressed_roster().expect("read fixture roster");
    let target = before.players.first().expect("a player").clone();

    let unused_before =
        player_points(common::progressed_gvas().expect("decode progressed-world"))
            .into_iter()
            .find(|(name, ..)| *name == target.name)
            .map(|(_, _, unused, _)| unused)
            .expect("target present");

    let bump = 7;
    let out = apply_edits(raw, &SaveEdits {
        player_edits: vec![PlayerEdit {
            instance_id: target.instance_id.clone(),
            level: Some(target.level + bump),
        }],
        pal_edits: Vec::new(),
    })
    .expect("apply_edits");

    let after = points_of(&out.level)
        .into_iter()
        .find(|(name, ..)| *name == target.name)
        .expect("target still present");

    assert_eq!(after.1, i64::from(target.level + bump), "level moved");
    assert_eq!(
        after.2,
        unused_before + i64::from(bump * STATUS_POINTS_PER_LEVEL),
        "seven levels should grant seven unspent status points"
    );
    assert_eq!(
        after.3,
        core_allocated_for(
            common::progressed_gvas().expect("decode progressed-world"),
            &target.name,
        ),
        "already-allocated points must not be touched"
    );
}

/// `player_points` for a save that only exists as bytes - an `apply_edits`
/// result, which by definition cannot come from the shared parse.
fn points_of(raw: &[u8]) -> Vec<(String, i64, i64, i64)> {
    let Ok(decompressed) = decompress(raw) else { return Vec::new() };
    let Ok(file) = read_gvas(&decompressed) else { return Vec::new() };
    player_points(&file)
}

fn core_allocated_for(file: &gvas::GvasFile, name: &str) -> i64 {
    player_points(file)
        .into_iter()
        .find(|(n, ..)| n == name)
        .map(|(_, _, _, core)| core)
        .unwrap_or_default()
}

/// A level change has to report which player files need patching, because
/// technology points are not in `Level.sav` at all.
#[test]
fn level_change_reports_the_player_uid_and_delta() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let before = common::progressed_roster().expect("read fixture roster");
    let target = before.players.first().expect("a player").clone();

    let out = apply_edits(raw, &SaveEdits {
        player_edits: vec![PlayerEdit {
            instance_id: target.instance_id.clone(),
            level: Some(target.level + 3),
        }],
        pal_edits: Vec::new(),
    })
    .expect("apply_edits");

    assert_eq!(out.level_deltas, vec![(target.player_uid, 3)]);
}

/// Setting the level to what it already is grants nothing.
#[test]
fn a_no_op_level_change_grants_no_points() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let before = common::progressed_roster().expect("read fixture roster");
    let target = before.players.first().expect("a player").clone();

    let out = apply_edits(raw, &SaveEdits {
        player_edits: vec![PlayerEdit {
            instance_id: target.instance_id.clone(),
            level: Some(target.level),
        }],
        pal_edits: Vec::new(),
    })
    .expect("apply_edits");

    assert!(out.level_deltas.is_empty(), "no level movement, no player files");
}

/// Pals have no status-point field and must not grow one.
#[test]
fn pal_level_changes_report_no_player_deltas() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let before = common::progressed_roster().expect("read fixture roster");
    let pal = before
        .players
        .iter()
        .flat_map(|p| p.pals.iter())
        .next()
        .or_else(|| before.base_pals.first())
        .expect("a pal")
        .clone();

    let out = apply_edits(raw, &SaveEdits {
        player_edits: Vec::new(),
        pal_edits: vec![palworld::save::edit::PalEdit {
            instance_id: pal.instance_id.clone(),
            level: Some(pal.level + 5),
            talent_hp: None,
            talent_shot: None,
            talent_defense: None,
            traits: None,
        }],
    })
    .expect("apply_edits");

    assert!(out.level_deltas.is_empty(), "a pal is not a player");
}

#[test]
fn grant_tech_points_adds_six_per_level_and_leaves_ancient_alone() {
    let dir = common::progressed_world();
    let roster = common::progressed_roster().expect("read fixture roster");
    let uid = &roster.players.first().expect("a player").player_uid;
    let stem = uid_to_filename(uid).expect("uid maps to a filename");
    let path = dir.join("Players").join(format!("{stem}.sav"));
    let raw = std::fs::read(&path).expect("read player save");

    let (tech_before, boss_before) = tech_points(&raw).expect("read points");

    let out = grant_tech_points(&raw, 4).expect("grant");
    let (tech_after, boss_after) = tech_points(&out).expect("read points back");

    assert_eq!(
        tech_after,
        tech_before + 4 * TECH_POINTS_PER_LEVEL,
        "four levels should grant 24 technology points"
    );
    assert_eq!(
        boss_after, boss_before,
        "Ancient Technology Points come from bosses, never from levelling"
    );
}

#[test]
fn grant_tech_points_round_trips_and_never_goes_negative() {
    let dir = common::progressed_world();
    let roster = common::progressed_roster().expect("read fixture roster");
    let uid = &roster.players.first().expect("a player").player_uid;
    let stem = uid_to_filename(uid).expect("uid maps to a filename");
    let raw = std::fs::read(dir.join("Players").join(format!("{stem}.sav")))
        .expect("read player save");

    let zero = grant_tech_points(&raw, 0).expect("zero grant");
    assert_eq!(
        tech_points(&zero).expect("read"),
        tech_points(&raw).expect("read"),
        "a zero delta must not move the totals"
    );

    // A huge negative delta floors at zero rather than wrapping.
    let drained = grant_tech_points(&raw, -100_000).expect("drain");
    assert_eq!(tech_points(&drained).expect("read").0, 0);
}
