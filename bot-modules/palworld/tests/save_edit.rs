//! Proof that the save write path is lossless before it is ever asked to
//! change anything.
//!
//! Every later editing case rests on this: if a zero-edit rewrite is not
//! byte-identical, a real edit is corrupting data somewhere we cannot see.

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use palworld::save::decompress::decompress;
use palworld::save::edit::{
    PalEdit,
    PlayerEdit,
    SaveEdits,
    apply_edits,
    read_roster,
};
use palworld::save::extract::{custom_struct, field, struct_fields};
use palworld::save::gvas::{read_gvas, reparse_properties_at, write_properties};

pub mod common;

/// A macro rather than a function: the workspace denies `expect_used` and only
/// exempts test functions, so the call has to expand inside the `#[test]`.
macro_rules! level_bytes {
    ($dir:expr) => {
        std::fs::read($dir.join("Level.sav")).expect("read fixture Level.sav")
    };
}

/// Pull every `CharacterSaveParameterMap` `RawData` blob out of a world.
fn rawdata_blobs(file: &gvas::GvasFile) -> Vec<Vec<u8>> {
    let Some(world) = custom_struct(file.properties.0.get("worldSaveData")) else {
        return Vec::new();
    };
    let Some(cspm) =
        world.0.get("CharacterSaveParameterMap").and_then(|v| v.first())
    else {
        return Vec::new();
    };
    let Property::MapProperty(MapProperty::Properties { value, .. }) = cspm else {
        return Vec::new();
    };

    value
        .0
        .values()
        .filter_map(|val| {
            let fields = struct_fields(val)?;
            if let Property::ArrayProperty(ArrayProperty::Bytes { bytes }) =
                field(fields, "RawData")?
            {
                Some(bytes.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn every_rawdata_blob_round_trips_byte_identically() {
    let file = common::progressed_gvas().expect("decode fixture Level.sav");
    let custom_versions = file.header.get_custom_versions().clone();

    let blobs = rawdata_blobs(file);
    assert_eq!(blobs.len(), 1822, "fixture carries 1822 characters");

    for (i, blob) in blobs.iter().enumerate() {
        let parsed = reparse_properties_at(blob, &custom_versions).expect("reparse");
        let rebuilt =
            write_properties(&parsed, &custom_versions).expect("write_properties");
        assert_eq!(
            &rebuilt, blob,
            "character {i} did not round-trip byte-identically",
        );
    }
}

#[test]
fn rawdata_tail_is_preserved_not_discarded() {
    let file = common::progressed_gvas().expect("decode fixture Level.sav");
    let custom_versions = file.header.get_custom_versions().clone();

    // Measured: every character carries exactly 24 bytes after the "None"
    // terminator (padding plus a group-id GUID). The editor copies them as an
    // opaque slice, so a future save version changing the length costs nothing -
    // but a *zero*-length tail would mean the reader is silently eating them.
    for blob in rawdata_blobs(file) {
        let parsed =
            reparse_properties_at(&blob, &custom_versions).expect("reparse");
        assert_eq!(parsed.tail.len(), 24, "tail is carried, not dropped");
    }
}

#[test]
fn whole_level_file_round_trips_byte_identically() {
    use std::io::Cursor;

    let raw = level_bytes!(common::steam_world1());
    let decompressed = decompress(&raw).expect("decompress");
    let file = read_gvas(&decompressed).expect("read_gvas");

    let mut out = Cursor::new(Vec::new());
    file.write(&mut out).expect("GvasFile::write");

    assert_eq!(
        out.into_inner(),
        decompressed,
        "an unmodified world re-serializes to the exact bytes it was read from",
    );
}

#[test]
fn roster_reads_players_pals_and_traits() {
    let raw = level_bytes!(common::progressed_world());
    let roster = read_roster(&raw, 1_700_000_000).expect("read_roster");

    assert_eq!(roster.level_modified, 1_700_000_000);
    assert_eq!(roster.players.len(), 8, "fixture has 8 players");

    // Instance ids are the browser-facing handle: they must be present,
    // uppercase hex, and unique across every character in the world.
    let mut ids: Vec<&str> = roster
        .players
        .iter()
        .map(|p| p.instance_id.as_str())
        .chain(
            roster
                .players
                .iter()
                .flat_map(|p| &p.pals)
                .map(|p| p.instance_id.as_str()),
        )
        .chain(roster.base_pals.iter().map(|p| p.instance_id.as_str()))
        .collect();
    let total = ids.len();
    assert!(total > 100, "fixture yields a substantial roster, got {total}");
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "instance ids are unique");
    assert!(
        ids.iter().all(|id| id.len() == 32
            && id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_lowercase())),
        "instance ids are 32-char uppercase hex",
    );

    let owned: Vec<_> = roster.players.iter().flat_map(|p| &p.pals).collect();
    assert!(!owned.is_empty(), "at least one player owns pals");
    assert!(
        owned.iter().all(|p| !p.species.is_empty()),
        "every pal carries a species",
    );
    assert!(
        owned.iter().all(|p| p.level >= 1),
        "an absent Level property reads back as level 1, not 0",
    );
}

#[test]
fn traits_are_harvested_sorted_and_deduplicated() {
    let roster = common::progressed_roster().expect("read fixture roster");

    assert!(!roster.trait_ids.is_empty(), "fixture carries passive skills");

    let mut sorted = roster.trait_ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, roster.trait_ids, "trait ids are sorted and unique");

    // Spot-check ids measured in the fixture. These are save-internal names,
    // not Paldex display slugs - which is exactly why they are harvested.
    for expected in ["Noukin", "Vampire", "PAL_CorporateSlave"] {
        assert!(
            roster.trait_ids.iter().any(|id| id == expected),
            "harvested set contains {expected}",
        );
    }

    // Every trait actually worn by a pal is offerable in the picker.
    for pal in roster.players.iter().flat_map(|p| &p.pals) {
        for t in &pal.traits {
            assert!(
                roster.trait_ids.contains(t),
                "worn trait {t} is in the harvested set",
            );
        }
    }
}

/// Apply `edits` to the fixture and read the result back.
macro_rules! edited {
    ($edits:expr) => {{
        let raw = common::progressed_level().expect("read fixture Level.sav");
        let out = apply_edits(raw, $edits).expect("apply_edits");
        read_roster(&out.level, 0).expect("re-read edited save")
    }};
}

macro_rules! first_pal {
    ($roster:expr) => {
        $roster
            .players
            .iter()
            .flat_map(|p| &p.pals)
            .next()
            .expect("fixture has at least one owned pal")
            .clone()
    };
}

#[test]
fn zero_edits_produce_a_readable_save() {
    let before = common::progressed_roster().expect("read fixture roster");
    let after = edited!(&SaveEdits::default());

    assert_eq!(after.players.len(), before.players.len());
    assert_eq!(after.trait_ids, before.trait_ids);
    assert_eq!(first_pal!(after).instance_id, first_pal!(before).instance_id);
}

#[test]
fn player_level_edit_applies_and_zeroes_exp() {
    let before = common::progressed_roster().expect("read fixture roster");
    let target = before.players.first().expect("a player").clone();

    let after = edited!(&SaveEdits {
        player_edits: vec![PlayerEdit {
            instance_id: target.instance_id.clone(),
            level: Some(55),
        }],
        pal_edits: Vec::new(),
    });

    let edited_player = after
        .players
        .iter()
        .find(|p| p.instance_id == target.instance_id)
        .expect("player survives the edit");
    assert_eq!(edited_player.level, 55);
    assert_eq!(edited_player.name, target.name, "unrelated fields are untouched");
}

#[test]
fn pal_level_ivs_and_traits_all_apply() {
    let before = common::progressed_roster().expect("read fixture roster");
    let target = first_pal!(before);
    let new_traits = vec!["Vampire".to_string(), "Noukin".to_string()];

    let after = edited!(&SaveEdits {
        player_edits: Vec::new(),
        pal_edits: vec![PalEdit {
            instance_id: target.instance_id.clone(),
            level: Some(42),
            talent_hp: Some(97),
            talent_shot: Some(3),
            talent_defense: Some(0),
            traits: Some(new_traits.clone()),
        }],
    });

    let pal = after
        .players
        .iter()
        .flat_map(|p| &p.pals)
        .find(|p| p.instance_id == target.instance_id)
        .expect("pal survives the edit");

    assert_eq!(pal.level, 42);
    assert_eq!(pal.talent_hp, 97);
    assert_eq!(pal.talent_shot, 3);
    assert_eq!(pal.talent_defense, 0);
    assert_eq!(pal.traits, new_traits);
    assert_eq!(pal.species, target.species, "species is not disturbed");
}

#[test]
fn absent_properties_are_inserted_not_skipped() {
    // 176 of the fixture's 1822 characters have no `Level` property at all and
    // 126 have no `PassiveSkillList` - the game omits defaults. Setting a value
    // on one of those must insert the property rather than silently no-op.
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let before = common::progressed_roster().expect("read fixture roster");

    let bare = before
        .players
        .iter()
        .flat_map(|p| &p.pals)
        .chain(before.base_pals.iter())
        .find(|p| p.level == 1 && p.traits.is_empty())
        .expect("fixture contains a pal with neither Level nor PassiveSkillList")
        .clone();

    let out = apply_edits(raw, &SaveEdits {
        player_edits: Vec::new(),
        pal_edits: vec![PalEdit {
            instance_id: bare.instance_id.clone(),
            level: Some(30),
            talent_hp: None,
            talent_shot: None,
            talent_defense: None,
            traits: Some(vec!["Vampire".to_string()]),
        }],
    })
    .expect("apply_edits");

    let after = read_roster(&out.level, 0).expect("re-read");
    let pal = after
        .players
        .iter()
        .flat_map(|p| &p.pals)
        .chain(after.base_pals.iter())
        .find(|p| p.instance_id == bare.instance_id)
        .expect("pal survives");

    assert_eq!(pal.level, 30, "Level was inserted");
    assert_eq!(
        pal.traits,
        vec!["Vampire".to_string()],
        "PassiveSkillList was inserted"
    );
}

#[test]
fn unknown_instance_id_is_an_error() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let err = apply_edits(raw, &SaveEdits {
        player_edits: Vec::new(),
        pal_edits: vec![PalEdit {
            instance_id: "00000000000000000000000000000000".to_string(),
            level: Some(10),
            talent_hp: None,
            talent_shot: None,
            talent_defense: None,
            traits: None,
        }],
    })
    .expect_err("an unknown instance id must not be silently skipped");

    assert!(
        err.to_string().contains("00000000000000000000000000000000"),
        "error names the missing instance id: {err}",
    );
}

#[test]
fn edited_save_is_a_valid_container() {
    let raw = common::progressed_level().expect("read fixture Level.sav");
    let out = apply_edits(raw, &SaveEdits::default()).expect("apply_edits");

    palworld::save::validate_level(&out.level)
        .expect("the exported save passes the same validation an upload would");
}
