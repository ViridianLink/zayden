use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::error::{PalworldError, Result};

const HEADER_LEN: usize = 12;

const MAGIC_ZLIB: &[u8; 3] = b"PlZ";
const MAGIC_CHUNKED: &[u8; 3] = b"CNK";

pub const TYPE_SINGLE: u8 = 0x31;
pub const TYPE_DOUBLE: u8 = 0x32;

const LEVEL: Compression = Compression::new(6);

pub fn compress(decompressed: &[u8], ty: u8) -> Result<Vec<u8>> {
    let body = match ty {
        TYPE_SINGLE => zlib(decompressed)?,
        TYPE_DOUBLE => zlib(&zlib(decompressed)?)?,
        other => {
            return Err(PalworldError::Save(format!(
                "cannot write save compression type 0x{other:02x}"
            )));
        },
    };

    let uncompressed_len = u32::try_from(decompressed.len()).map_err(|e| {
        PalworldError::Save(format!(
            "save is too large to describe in the header: {e}"
        ))
    })?;
    let compressed_len = u32::try_from(body.len()).map_err(|e| {
        PalworldError::Save(format!(
            "compressed save is too large for the header: {e}"
        ))
    })?;

    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(&compressed_len.to_le_bytes());
    out.extend_from_slice(MAGIC_ZLIB);
    out.push(ty);
    out.extend_from_slice(&body);
    Ok(out)
}

pub fn source_type_byte(raw: &[u8]) -> Result<u8> {
    let at = if raw.get(8..11) == Some(MAGIC_CHUNKED) { HEADER_LEN } else { 0 };
    raw.get(at + 11)
        .copied()
        .ok_or_else(|| PalworldError::Save("truncated save header".into()))
}

fn zlib(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), LEVEL);
    encoder
        .write_all(bytes)
        .map_err(|e| PalworldError::Save(format!("zlib compress failed: {e}")))?;
    encoder
        .finish()
        .map_err(|e| PalworldError::Save(format!("zlib compress failed: {e}")))
}
