use std::collections::HashMap;

use gvas::GvasFile;
use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::int_property::BytePropertyValue;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use gvas::types::Guid;
use gvas::types::map::HashableIndexMap;

use crate::error::{PalworldError, Result};
use crate::model::{Gender, OwnedPal};

#[derive(Debug, Default)]
pub struct ExtractedWorld {
    pub players: HashMap<String, PlayerInfo>,
    pub pals: HashMap<String, Vec<OwnedPal>>,
    pub base_pals: Vec<BasePal>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerInfo {
    pub name: String,
    pub level: i64,
    pub exp: i64,
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
                out.players.insert(uid, PlayerInfo {
                    name,
                    level: int_field(save_param, "Level"),
                    exp: int_field(save_param, "Exp"),
                });
            }
            continue;
        }

        let Some(pal) = owned_pal(save_param) else { continue };

        if let Some(owner) = owner_uid(save_param) {
            out.pals.entry(owner).or_default().push(pal);
        } else if let Some(last_owner) = old_owner_last(save_param) {
            out.base_pals.push(BasePal { last_owner, pal });
        }
    }

    Ok(out)
}

#[must_use]
pub const fn struct_fields(
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

#[must_use]
pub fn custom_struct(
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

#[must_use]
pub fn field<'a>(
    fields: &'a HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> Option<&'a Property> {
    fields.0.get(name).and_then(|v| v.first())
}

#[must_use]
pub fn int_field(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> i64 {
    match field(fields, name) {
        Some(Property::IntProperty(i)) => i64::from(i.value),
        Some(Property::Int64Property(i)) => i.value,
        Some(Property::ByteProperty(b)) => match b.value {
            BytePropertyValue::Byte(v) => i64::from(v),
            BytePropertyValue::Namespaced(_) => 0,
        },
        _ => 0,
    }
}

#[must_use]
pub fn bool_field(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> bool {
    matches!(field(fields, name), Some(Property::BoolProperty(b)) if b.value)
}

fn is_player(fields: &HashableIndexMap<String, Vec<Property>>) -> bool {
    bool_field(fields, "IsPlayer")
}

fn stars(fields: &HashableIndexMap<String, Vec<Property>>) -> u8 {
    let rank = int_field(fields, "Rank");
    u8::try_from(rank.saturating_sub(1).clamp(0, 4)).unwrap_or(0)
}

fn character_id(fields: &HashableIndexMap<String, Vec<Property>>) -> Option<String> {
    if let Some(Property::NameProperty(n)) = field(fields, "CharacterID") {
        n.value.clone()
    } else {
        None
    }
}

#[must_use]
pub fn nickname(fields: &HashableIndexMap<String, Vec<Property>>) -> Option<String> {
    if let Some(Property::StrProperty(s)) = field(fields, "NickName") {
        s.value.clone()
    } else {
        None
    }
}

#[must_use]
pub fn gender(fields: &HashableIndexMap<String, Vec<Property>>) -> Gender {
    if let Some(Property::EnumProperty(e)) = field(fields, "Gender") {
        Gender::parse(&e.value)
    } else {
        Gender::Unknown
    }
}

#[must_use]
pub fn owned_pal(
    save_param: &HashableIndexMap<String, Vec<Property>>,
) -> Option<OwnedPal> {
    let species = character_id(save_param)?;
    Some(OwnedPal {
        is_alpha: species.to_ascii_uppercase().starts_with("BOSS_"),
        is_lucky: bool_field(save_param, "IsRare"),
        stars: stars(save_param),
        species,
        gender: gender(save_param),
        nickname: nickname(save_param),
    })
}

#[must_use]
pub fn owner_uid(
    fields: &HashableIndexMap<String, Vec<Property>>,
) -> Option<String> {
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

#[must_use]
pub fn key_player_uid(key: &Property) -> Option<String> {
    let fields = struct_fields(key)?;
    let bytes = guid_bytes(field(fields, "PlayerUId")?)?;
    (bytes != [0u8; 16]).then(|| hex_upper(&bytes))
}

#[must_use]
pub fn key_instance_id(key: &Property) -> Option<String> {
    let fields = struct_fields(key)?;
    let bytes = guid_bytes(field(fields, "InstanceId")?)?;
    (bytes != [0u8; 16]).then(|| hex_upper(&bytes))
}

#[must_use]
pub const fn guid_bytes(prop: &Property) -> Option<[u8; 16]> {
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

#[must_use]
pub fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02X}");
        s
    })
}
