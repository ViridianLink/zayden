//! `save::decompress` - the `.sav` outer-wrapper codec.
//!
//! The real `PlM`/Oodle `Level.sav` decompress is asserted when the save
//! folder is present (mirroring the gated live tests); the synthetic `PlZ`
//! round-trip and the malformed-header rejections always run so `cargo test`
//! stays green offline.

use std::io::Write;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use palworld::error::PalworldError;
use palworld::save::decompress::decompress;

const GVAS_MAGIC: &[u8; 4] = b"GVAS";

/// Build a Palworld `PlZ` wrapper (`u32` uncompressed len, `u32` compressed
/// len, `PlZ`, type byte) around a zlib-compressed payload.
fn plz_blob(payload: &[u8], type_byte: u8) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(payload);
    let compressed = encoder.finish().unwrap_or_default();

    let mut blob = Vec::new();
    blob.extend(u32::try_from(payload.len()).unwrap_or(0).to_le_bytes());
    blob.extend(u32::try_from(compressed.len()).unwrap_or(0).to_le_bytes());
    blob.extend(b"PlZ");
    blob.push(type_byte);
    blob.extend(&compressed);
    blob
}

#[test]
fn plz_single_round_trips() {
    let payload = b"GVAS synthetic world payload \x00\x01\x02 end";
    let blob = plz_blob(payload, 0x31);
    assert_eq!(decompress(&blob).unwrap(), payload);
}

#[test]
fn unknown_magic_is_rejected() {
    let mut blob = Vec::new();
    blob.extend(4u32.to_le_bytes());
    blob.extend(4u32.to_le_bytes());
    blob.extend(b"XxX");
    blob.push(0x31);
    blob.extend(b"junk");
    assert!(matches!(decompress(&blob), Err(PalworldError::Save(_))));
}

#[test]
fn none_type_is_rejected() {
    let mut blob = Vec::new();
    blob.extend(4u32.to_le_bytes());
    blob.extend(4u32.to_le_bytes());
    blob.extend(b"PlZ");
    blob.push(0x30);
    blob.extend(b"data");
    assert!(matches!(decompress(&blob), Err(PalworldError::Save(_))));
}

#[test]
fn truncated_header_is_rejected() {
    assert!(matches!(decompress(&[0u8; 5]), Err(PalworldError::Save(_))));
}

#[test]
fn real_oodle_level_sav_decompresses_to_gvas() {
    // The save dir lives at the workspace root, two levels up from the crate.
    let level = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../056C426C55974CFCA115EB695A224F67/Level.sav");
    let Ok(raw) = std::fs::read(&level) else {
        eprintln!("skipping: real save not present at {}", level.display());
        return;
    };

    let out = decompress(&raw).expect("real PlM Level.sav decompresses");
    assert_eq!(
        out.get(..4),
        Some(GVAS_MAGIC.as_slice()),
        "decompressed payload starts with GVAS"
    );
}
