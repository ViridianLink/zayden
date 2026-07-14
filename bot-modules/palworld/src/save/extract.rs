use std::collections::HashMap;

use gvas::GvasFile;
use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use gvas::types::Guid;
use gvas::types::map::HashableIndexMap;

use crate::error::{PalworldError, Result};
use crate::model::{Gender, OwnedPal};

#[derive(Debug, Default)]
pub struct ExtractedWorld {
    pub player_names: HashMap<String, String>,
    pub pals: HashMap<String, Vec<OwnedPal>>,
    pub base_pals: Vec<BasePal>,
}

#[derive(Debug, Clone)]
pub struct BasePal {
    pub last_owner: String,
    pub pal: OwnedPal,
}

pub fn extract(level: &GvasFile) -> Result<ExtractedWorld> {
    let custom_versions = level.header.get_custom_versions().clone();

    let world = custom_struct(level.properties.0.get("worldSaveData"))
        .ok_or_else(|| PalworldError::Gvas("missing worldSaveData struct".into()))?;
    let cspm = world
        .0
        .get("CharacterSaveParameterMap")
        .and_then(|v| v.first())
        .ok_or_else(|| {
            PalworldError::Gvas("missing CharacterSaveParameterMap".into())
        })?;
    let MapProperty::Properties { value, .. } = as_map(cspm)? else {
        return Err(PalworldError::Gvas(
            "CharacterSaveParameterMap has unexpected shape".into(),
        ));
    };

    let mut out = ExtractedWorld::default();
    for (key, val) in &value.0 {
        let Some(raw) = rawdata_bytes(val) else { continue };
        let parsed = match super::gvas::reparse_properties(raw, &custom_versions) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("skipping unparseable CharacterSaveParameter: {e}");
                continue;
            },
        };

        let Some(save_param) = parsed
            .iter()
            .find(|(k, _)| k == "SaveParameter")
            .and_then(|(_, p)| struct_fields(p))
        else {
            continue;
        };

        if is_player(save_param) {
            if let (Some(uid), Some(name)) =
                (key_player_uid(key), nickname(save_param))
            {
                out.player_names.insert(uid, name);
            }
            continue;
        }

        let Some(species) = character_id(save_param) else { continue };
        let pal = OwnedPal {
            species,
            gender: gender(save_param),
            nickname: nickname(save_param),
        };

        if let Some(owner) = owner_uid(save_param) {
            out.pals.entry(owner).or_default().push(pal);
        } else if let Some(last_owner) = old_owner_last(save_param) {
            out.base_pals.push(BasePal { last_owner, pal });
        }
    }

    Ok(out)
}

pub(crate) const fn struct_fields(
    prop: &Property,
) -> Option<&HashableIndexMap<String, Vec<Property>>> {
    let value = if let Property::StructProperty(s) = prop {
        &s.value
    } else if let Property::StructPropertyValue(v) = prop {
        v
    } else {
        return None;
    };
    if let StructPropertyValue::CustomStruct(m) = value { Some(m) } else { None }
}

pub(crate) fn custom_struct(
    prop: Option<&Property>,
) -> Option<&HashableIndexMap<String, Vec<Property>>> {
    struct_fields(prop?)
}

fn as_map(prop: &Property) -> Result<&MapProperty> {
    if let Property::MapProperty(m) = prop {
        Ok(m)
    } else {
        Err(PalworldError::Gvas("expected MapProperty".into()))
    }
}

pub(crate) fn field<'a>(
    fields: &'a HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> Option<&'a Property> {
    fields.0.get(name).and_then(|v| v.first())
}

fn is_player(fields: &HashableIndexMap<String, Vec<Property>>) -> bool {
    matches!(field(fields, "IsPlayer"), Some(Property::BoolProperty(b)) if b.value)
}

fn character_id(fields: &HashableIndexMap<String, Vec<Property>>) -> Option<String> {
    if let Some(Property::NameProperty(n)) = field(fields, "CharacterID") {
        n.value.clone()
    } else {
        None
    }
}

fn nickname(fields: &HashableIndexMap<String, Vec<Property>>) -> Option<String> {
    if let Some(Property::StrProperty(s)) = field(fields, "NickName") {
        s.value.clone()
    } else {
        None
    }
}

fn gender(fields: &HashableIndexMap<String, Vec<Property>>) -> Gender {
    if let Some(Property::EnumProperty(e)) = field(fields, "Gender") {
        Gender::parse(&e.value)
    } else {
        Gender::Unknown
    }
}

fn owner_uid(fields: &HashableIndexMap<String, Vec<Property>>) -> Option<String> {
    let bytes = guid_bytes(field(fields, "OwnerPlayerUId")?)?;
    (bytes != [0u8; 16]).then(|| hex_upper(&bytes))
}

fn old_owner_last(
    fields: &HashableIndexMap<String, Vec<Property>>,
) -> Option<String> {
    let Property::ArrayProperty(ArrayProperty::Structs { structs, .. }) =
        field(fields, "OldOwnerPlayerUIds")?
    else {
        return None;
    };
    let StructPropertyValue::Guid(Guid(bytes)) = structs.last()? else {
        return None;
    };
    (*bytes != [0u8; 16]).then(|| hex_upper(bytes))
}

fn key_player_uid(key: &Property) -> Option<String> {
    let fields = struct_fields(key)?;
    let bytes = guid_bytes(field(fields, "PlayerUId")?)?;
    (bytes != [0u8; 16]).then(|| hex_upper(&bytes))
}

const fn guid_bytes(prop: &Property) -> Option<[u8; 16]> {
    let value = if let Property::StructProperty(s) = prop {
        &s.value
    } else if let Property::StructPropertyValue(v) = prop {
        v
    } else {
        return None;
    };
    if let StructPropertyValue::Guid(Guid(bytes)) = value {
        Some(*bytes)
    } else {
        None
    }
}

fn rawdata_bytes(val: &Property) -> Option<&[u8]> {
    let fields = struct_fields(val)?;
    if let Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) =
        field(fields, "RawData")
    {
        Some(bytes)
    } else {
        None
    }
}

pub(crate) fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02X}");
        s
    })
}
