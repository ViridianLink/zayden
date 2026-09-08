//! Proof that the save read/write path is lossless.
//!
//! `read_gvas` -> `write_properties` has to reproduce its input byte for byte;
//! anything built on top of it is corrupting data somewhere we cannot see if it
//! does not.

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use palworld::save::decompress::decompress;
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
    // terminator (padding plus a group-id GUID). They are carried as an opaque
    // slice, so a future save version changing the length costs nothing - but a
    // *zero*-length tail would mean the reader is silently eating them.
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
