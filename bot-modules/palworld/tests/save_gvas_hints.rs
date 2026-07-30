//! Hint recovery in `save::gvas`.
//!
//! A Palworld save embeds struct bodies with no type tag, so `gvas` needs a hint
//! per struct-keyed or struct-valued map. Three game updates in a row have added
//! a new one, and each time the *entire* save became undecodable - not just the
//! new field. These tests pin the recovery that stops the next update from
//! taking `/palworld progress` down with it.

use std::path::Path;

use palworld::save::gvas::{hints, read_gvas, read_inferring};

pub mod common;
use common::{progressed_world, storage_world};

/// The decompressed GVAS of the one fixture save that needs the newest hint.
fn player_save() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path =
        storage_world().join("Players").join("B0726C28000000000000000000000000.sav");
    decompressed(&path)
}

fn decompressed(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(palworld::save::decompress::decompress(&std::fs::read(path)?)?)
}

/// Decodes with the shipped table alone - no inference allowed to cover for it.
fn strictly_decodes(bytes: &[u8]) -> Result<(), gvas::error::Error> {
    gvas::GvasFile::read_with_hints(
        &mut std::io::Cursor::new(bytes),
        gvas::game_version::GameVersion::Default,
        &hints(),
    )
    .map(drop)
}

/// The shipped table decodes the fixture outright - the inference below is a
/// safety net, not the mechanism.
#[test]
fn the_shipped_hint_table_is_complete_for_current_saves() {
    let bytes = player_save().expect("fixture");
    assert!(read_inferring(&bytes, hints()).is_ok());

    // No inference needed means the same read succeeds with the table alone.
    assert!(strictly_decodes(&bytes).is_ok(), "shipped hints alone");
}

/// Every `worldSaveData.*` hint lives in `Level.sav`, so the player save above
/// exercises almost none of the table. Both world fixtures must decode strictly
/// too, or a stale entry there only surfaces as a runtime inference warning.
#[test]
fn the_shipped_hint_table_is_complete_for_world_saves() {
    for world in [progressed_world(), storage_world()] {
        let path = world.join("Level.sav");
        let bytes = decompressed(&path).expect("fixture");

        if let Err(e) = strictly_decodes(&bytes) {
            panic!("shipped hints alone on {}: {e}", path.display());
        }
    }
}

/// `InvaderDeclarationSaveData` only appears in a world where a raid has been
/// declared, which no committed fixture has, so the two strict tests above cannot
/// cover it. It was recovered by inference from a live server save (the reader
/// logged `... ValidatedStartPointIds.SetProperty.StructProperty = Guid`); pin it
/// here so the entry is not dropped as unused.
#[test]
fn the_invader_declaration_set_is_hinted() {
    let path = "worldSaveData.StructProperty.InvaderDeclarationSaveData.\
                StructProperty.ValidatedStartPointIds.SetProperty.StructProperty";

    assert_eq!(hints().get(path).map(String::as_str), Some("Guid"));
}

/// With an empty table, the reader has to discover
/// `FoundTreasureMapPointMap`'s key *and* value layouts - a GUID and a
/// named-field struct - which also exercises backtracking, since the value only
/// decodes on the second candidate.
#[test]
fn a_missing_hint_is_recovered_without_it() {
    let bytes = player_save().expect("fixture");

    let recovered = read_inferring(&bytes, []).expect("inferred");
    let expected = read_gvas(&bytes).expect("shipped hints");

    assert_eq!(
        recovered.properties, expected.properties,
        "inferred hints must reconstruct exactly what the real ones do",
    );
}

/// Inference must not paper over a corrupt save: truncated bytes still fail.
#[test]
fn inference_does_not_rescue_a_broken_save() {
    let bytes = player_save().expect("fixture");
    let truncated = bytes.get(..bytes.len() / 2).expect("half of the save");

    assert!(read_inferring(truncated, []).is_err());
    assert!(read_gvas(truncated).is_err());
    assert!(read_gvas(b"not a save at all").is_err());
}
