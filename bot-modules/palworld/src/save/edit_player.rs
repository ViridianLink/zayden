use std::io::Cursor;

use gvas::properties::Property;
use gvas::properties::int_property::IntProperty;

use super::extract::{field, struct_fields_mut};
use super::{compress, decompress, gvas as save_gvas};
use crate::error::{PalworldError, Result};

pub const TECH_POINTS_PER_LEVEL: i32 = 6;

pub fn grant_tech_points(player_bytes: &[u8], level_delta: i32) -> Result<Vec<u8>> {
    let ty = compress::source_type_byte(player_bytes)?;
    let decompressed = decompress::decompress(player_bytes)?;
    let mut file = save_gvas::read_gvas(&decompressed)?;

    let save_data = file
        .properties
        .0
        .get_mut("SaveData")
        .and_then(struct_fields_mut)
        .ok_or_else(|| {
            PalworldError::Edit(
                "not a player save: missing SaveData struct".to_string(),
            )
        })?;

    let current = if let Some(Property::IntProperty(p)) =
        field(save_data, "TechnologyPoint")
    {
        p.value
    } else {
        0
    };
    let granted = level_delta.saturating_mul(TECH_POINTS_PER_LEVEL);
    let next = current.saturating_add(granted).max(0);

    match save_data.0.get_mut("TechnologyPoint").and_then(|v| v.first_mut()) {
        Some(existing) => *existing = Property::IntProperty(IntProperty::new(next)),
        None => {
            let _ = save_data.0.insert("TechnologyPoint".to_string(), vec![
                Property::IntProperty(IntProperty::new(next)),
            ]);
        },
    }

    let mut out = Cursor::new(Vec::new());
    file.write(&mut out).map_err(|e| PalworldError::Gvas(e.to_string()))?;
    compress::compress(&out.into_inner(), ty)
}

pub fn tech_points(player_bytes: &[u8]) -> Result<(i32, i32)> {
    let decompressed = decompress::decompress(player_bytes)?;
    let file = save_gvas::read_gvas(&decompressed)?;
    let save_data = super::extract::custom_struct(file.properties.0.get("SaveData"))
        .ok_or_else(|| {
            PalworldError::Gvas(
                "not a player save: missing SaveData struct".to_string(),
            )
        })?;

    let read = |name: &str| {
        if let Some(Property::IntProperty(p)) = field(save_data, name) {
            p.value
        } else {
            0
        }
    };
    Ok((read("TechnologyPoint"), read("bossTechnologyPoint")))
}
