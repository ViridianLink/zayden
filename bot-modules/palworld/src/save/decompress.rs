use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::{PalworldError, Result};

const HEADER_LEN: usize = 12;

const MAGIC_OODLE: &[u8; 3] = b"PlM";
const MAGIC_ZLIB: &[u8; 3] = b"PlZ";

const TYPE_NONE: u8 = 0x30;
const TYPE_SINGLE: u8 = 0x31;
const TYPE_DOUBLE: u8 = 0x32;

fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let arr: [u8; 4] = bytes
        .get(at..at + 4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| PalworldError::Save("truncated header".into()))?;
    Ok(u32::from_le_bytes(arr))
}

pub fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
    let uncompressed_len = read_u32(bytes, 0)? as usize;
    let _compressed_len = read_u32(bytes, 4)?;

    let magic = bytes
        .get(8..11)
        .ok_or_else(|| PalworldError::Save("truncated header".into()))?;
    let ty = *bytes
        .get(11)
        .ok_or_else(|| PalworldError::Save("truncated header".into()))?;
    let body = bytes
        .get(HEADER_LEN..)
        .ok_or_else(|| PalworldError::Save("missing compressed body".into()))?;

    let out = match (magic, ty) {
        (m, TYPE_SINGLE) if m == MAGIC_OODLE => {
            decompress_oodle(body, uncompressed_len)?
        },
        (m, TYPE_SINGLE) if m == MAGIC_ZLIB => {
            decompress_zlib(body, uncompressed_len, 1)?
        },
        (m, TYPE_DOUBLE) if m == MAGIC_ZLIB => {
            decompress_zlib(body, uncompressed_len, 2)?
        },
        (m, TYPE_NONE) if m == MAGIC_OODLE || m == MAGIC_ZLIB => {
            return Err(PalworldError::Save(
                "uncompressed (0x30) saves are not supported".into(),
            ));
        },
        _ => {
            return Err(PalworldError::Save(format!(
                "unknown save format: magic={magic:?} type=0x{ty:02x}"
            )));
        },
    };

    if out.len() != uncompressed_len {
        return Err(PalworldError::Save(format!(
            "decompressed length {} != declared {uncompressed_len}",
            out.len()
        )));
    }

    Ok(out)
}

fn decompress_oodle(body: &[u8], uncompressed_len: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; uncompressed_len];
    let written =
        oozextract::Extractor::new().read_from_slice(body, &mut out).map_err(
            |e| PalworldError::Save(format!("oodle decompress failed: {e:?}")),
        )?;
    out.truncate(written);
    Ok(out)
}

fn decompress_zlib(
    body: &[u8],
    uncompressed_len: usize,
    passes: u8,
) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(uncompressed_len);
    match passes {
        1 => {
            ZlibDecoder::new(body).read_to_end(&mut out).map_err(|e| {
                PalworldError::Save(format!("zlib decompress failed: {e}"))
            })?;
        },
        _ => {
            let inner = ZlibDecoder::new(body);
            ZlibDecoder::new(inner).read_to_end(&mut out).map_err(|e| {
                PalworldError::Save(format!("zlib decompress failed: {e}"))
            })?;
        },
    }
    Ok(out)
}
