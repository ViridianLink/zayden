use std::collections::HashMap;
use std::path::Path;

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::struct_property::StructPropertyValue;

use super::decompress;
use super::extract::{field, owned_pal, owner_uid, struct_fields};
use crate::error::Result;
use crate::model::OwnedPal;

#[must_use]
pub fn load_all(save_dir: &Path) -> HashMap<String, Vec<OwnedPal>> {
    let mut out: HashMap<String, Vec<OwnedPal>> = HashMap::new();

    let Ok(entries) = std::fs::read_dir(save_dir.join("Players")) else {
        return out;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with("_dps.sav") {
            continue;
        }

        let parsed = std::fs::read(&path)
            .map_err(crate::error::PalworldError::from)
            .and_then(|raw| parse(&raw));

        match parsed {
            Ok(pals) => {
                for (uid, mut owned) in pals {
                    out.entry(uid).or_default().append(&mut owned);
                }
            },
            Err(e) => tracing::warn!(
                error = %e,
                file = name,
                "palworld: skipping unreadable Pal storage save",
            ),
        }
    }

    out
}

pub fn parse(raw: &[u8]) -> Result<HashMap<String, Vec<OwnedPal>>> {
    let file = super::gvas::read_gvas(&decompress::decompress(raw)?)?;

    let Some(Property::ArrayProperty(ArrayProperty::Structs { structs, .. })) =
        file.properties.0.get("SaveParameterArray")
    else {
        return Ok(HashMap::new());
    };

    let mut out: HashMap<String, Vec<OwnedPal>> = HashMap::new();
    for slot in structs {
        let StructPropertyValue::CustomStruct(fields) = slot else { continue };
        let Some(save_param) =
            field(fields, "SaveParameter").and_then(struct_fields)
        else {
            continue;
        };
        let (Some(owner), Some(pal)) =
            (owner_uid(save_param), owned_pal(save_param))
        else {
            continue;
        };
        out.entry(owner).or_default().push(pal);
    }

    Ok(out)
}
