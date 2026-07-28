//! Round-trip proof for the save container writer.
//!
//! The source fixtures are Oodle-compressed (`PlM1`) and there is no Oodle
//! encoder, so `compress` emits zlib (`PlZ`). These cases prove the container we
//! emit is self-consistent - that our own `decompress` reads back exactly what
//! went in, and that the type byte survives. Whether the *game* accepts `PlZ` is
//! a manual check, documented in the plan.

use palworld::save::compress::{compress, source_type_byte};
use palworld::save::decompress::decompress;

pub mod common;

const MAGIC_ZLIB: &[u8; 3] = b"PlZ";
const HEADER_LEN: usize = 12;

/// A macro rather than a function: the workspace denies `expect_used` and only
/// exempts test functions, so the call has to expand inside the `#[test]`.
macro_rules! level_bytes {
    ($dir:expr) => {
        std::fs::read($dir.join("Level.sav")).expect("read fixture Level.sav")
    };
}

#[test]
fn compress_round_trips_through_decompress() {
    for raw in [
        level_bytes!(common::steam_world1()),
        level_bytes!(common::progressed_world()),
    ] {
        let decompressed = decompress(&raw).expect("decompress fixture");
        let ty = source_type_byte(&raw).expect("source type byte");

        let recompressed = compress(&decompressed, ty).expect("compress");
        let again = decompress(&recompressed).expect("decompress our own output");

        assert_eq!(again, decompressed, "compress -> decompress is lossless");
    }
}

#[test]
fn compressed_header_is_plz_with_the_source_type_byte() {
    let raw = level_bytes!(common::steam_world1());
    let decompressed = decompress(&raw).expect("decompress fixture");
    let ty = source_type_byte(&raw).expect("source type byte");
    assert_eq!(ty, 0x31, "fixture is single-compressed");

    let out = compress(&decompressed, ty).expect("compress");

    assert!(out.len() > HEADER_LEN, "output carries a header and a body");
    assert_eq!(out.get(8..11), Some(MAGIC_ZLIB.as_slice()), "magic is PlZ");
    assert_eq!(out.get(11).copied(), Some(ty), "type byte is preserved");

    let le_u32 = |at: usize| {
        u32::from_le_bytes(
            out.get(at..at + 4)
                .and_then(|s| s.try_into().ok())
                .expect("4 header bytes"),
        )
    };
    assert_eq!(le_u32(0) as usize, decompressed.len());
    assert_eq!(le_u32(4) as usize, out.len() - HEADER_LEN);
}

#[test]
fn unsupported_type_byte_is_rejected() {
    let err = compress(b"anything", 0x30).expect_err("0x30 is not supported");
    assert!(
        err.to_string().contains("0x30"),
        "error names the offending type byte: {err}"
    );
}
