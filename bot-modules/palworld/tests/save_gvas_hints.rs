//! Hint recovery in `save::gvas`.
//!
//! A Palworld save embeds struct bodies with no type tag, so `gvas` needs a hint
//! per struct-keyed or struct-valued map. Three game updates in a row have added
//! a new one, and each time the *entire* save became undecodable - not just the
//! new field. These tests pin the recovery that stops the next update from
//! taking `/palworld progress` down with it.

use palworld::save::gvas::{hints, read_gvas, read_inferring};

pub mod common;
use common::storage_world;

/// The decompressed GVAS of the one fixture save that needs the newest hint.
fn player_save() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path =
        storage_world().join("Players").join("B0726C28000000000000000000000000.sav");
    Ok(palworld::save::decompress::decompress(&std::fs::read(path)?)?)
}

/// The shipped table decodes the fixture outright - the inference below is a
/// safety net, not the mechanism.
#[test]
fn the_shipped_hint_table_is_complete_for_current_saves() {
    let bytes = player_save().expect("fixture");
    assert!(read_inferring(&bytes, hints()).is_ok());

    // No inference needed means the same read succeeds with the table alone.
    let strict = gvas::GvasFile::read_with_hints(
        &mut std::io::Cursor::new(&bytes),
        gvas::game_version::GameVersion::Default,
        &hints(),
    );
    assert!(strict.is_ok(), "shipped hints alone: {:?}", strict.err());
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
