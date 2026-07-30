use std::collections::HashMap;
use std::io::Cursor;

use gvas::GvasFile;
use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::int_property::{
    ByteProperty,
    BytePropertyValue,
    Int64Property,
    UInt16Property,
};
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use gvas::types::Guid;
use gvas::types::map::HashableIndexMap;
use serde::{Deserialize, Serialize};

use super::extract::{
    bool_field,
    custom_struct,
    field,
    int_field,
    key_instance_id,
    key_player_uid,
    nickname,
    owner_uid,
    struct_fields,
};
use super::{compress, decompress, gvas as save_gvas};
use crate::error::{PalworldError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveRoster {
    pub level_modified: i64,
    pub trait_ids: Vec<String>,
    pub players: Vec<SavePlayer>,
    pub base_pals: Vec<SavePal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePlayer {
    pub instance_id: String,
    pub player_uid: String,
    pub name: String,
    pub level: i32,
    pub pals: Vec<SavePal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePal {
    pub instance_id: String,
    pub species: String,
    pub nickname: Option<String>,
    pub gender: String,
    pub stars: u8,
    pub is_lucky: bool,
    pub is_alpha: bool,
    pub level: i32,
    pub talent_hp: u8,
    pub talent_shot: u8,
    pub talent_defense: u8,
    pub traits: Vec<String>,
}

struct Entry {
    instance_id: String,
    player_uid: Option<String>,
    is_player: bool,
    owner: Option<String>,
    pal: SavePal,
    name: String,
}

pub fn read_roster(level_bytes: &[u8], level_modified: i64) -> Result<SaveRoster> {
    let decompressed = decompress::decompress(level_bytes)?;
    let file = save_gvas::read_gvas(&decompressed)?;
    let custom_versions = file.header.get_custom_versions().clone();

    let entries = decode_entries(&file, &custom_versions)?;

    let mut trait_ids: Vec<String> =
        entries.iter().flat_map(|e| e.pal.traits.iter().cloned()).collect();
    trait_ids.sort_unstable();
    trait_ids.dedup();

    let mut players: Vec<SavePlayer> = entries
        .iter()
        .filter(|e| e.is_player)
        .map(|e| SavePlayer {
            instance_id: e.instance_id.clone(),
            player_uid: e.player_uid.clone().unwrap_or_default(),
            name: e.name.clone(),
            level: e.pal.level,
            pals: Vec::new(),
        })
        .collect();

    let mut base_pals = Vec::new();
    for entry in entries.iter().filter(|e| !e.is_player) {
        match entry
            .owner
            .as_deref()
            .and_then(|uid| players.iter_mut().find(|p| p.player_uid == uid))
        {
            Some(player) => player.pals.push(entry.pal.clone()),
            None => base_pals.push(entry.pal.clone()),
        }
    }

    players.sort_by_key(|p| p.name.to_lowercase());
    for player in &mut players {
        player.pals.sort_by(|a, b| a.species.cmp(&b.species));
    }
    base_pals.sort_by(|a, b| a.species.cmp(&b.species));

    Ok(SaveRoster { level_modified, trait_ids, players, base_pals })
}

fn decode_entries(
    file: &GvasFile,
    custom_versions: &HashableIndexMap<Guid, u32>,
) -> Result<Vec<Entry>> {
    let mut out = Vec::new();
    for (key, val) in character_map(file)? {
        let Some(instance_id) = key_instance_id(key) else { continue };
        let Some(raw) = rawdata(val) else { continue };
        let parsed = match save_gvas::reparse_properties_at(raw, custom_versions) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("skipping unparseable character {instance_id}: {e}");
                continue;
            },
        };
        let Some(save_param) = parsed
            .properties
            .iter()
            .find(|(k, _)| k == "SaveParameter")
            .and_then(|(_, p)| struct_fields(p))
        else {
            continue;
        };

        let is_player = bool_field(save_param, "IsPlayer");
        let name = nickname(save_param).unwrap_or_default();
        out.push(Entry {
            player_uid: key_player_uid(key),
            is_player,
            owner: owner_uid(save_param),
            pal: read_pal(&instance_id, save_param),
            name,
            instance_id,
        });
    }
    Ok(out)
}

fn read_pal(
    instance_id: &str,
    save_param: &HashableIndexMap<String, Vec<Property>>,
) -> SavePal {
    let species =
        if let Some(Property::NameProperty(n)) = field(save_param, "CharacterID") {
            n.value.clone().unwrap_or_default()
        } else {
            String::new()
        };
    let gender = if let Some(Property::EnumProperty(e)) = field(save_param, "Gender")
    {
        e.value.clone()
    } else {
        String::new()
    };

    SavePal {
        instance_id: instance_id.to_string(),
        is_alpha: species.to_ascii_uppercase().starts_with("BOSS_"),
        species,
        gender,
        nickname: nickname(save_param),
        stars: clamp_u8(int_field(save_param, "Rank").saturating_sub(1), 4),
        is_lucky: bool_field(save_param, "IsRare"),
        level: i32::try_from(int_field(save_param, "Level")).unwrap_or(1).max(1),
        talent_hp: clamp_u8(int_field(save_param, "Talent_HP"), 100),
        talent_shot: clamp_u8(int_field(save_param, "Talent_Shot"), 100),
        talent_defense: clamp_u8(int_field(save_param, "Talent_Defense"), 100),
        traits: traits(save_param),
    }
}

fn traits(save_param: &HashableIndexMap<String, Vec<Property>>) -> Vec<String> {
    if let Some(Property::ArrayProperty(ArrayProperty::Names { names })) =
        field(save_param, "PassiveSkillList")
    {
        names.iter().flatten().cloned().collect()
    } else {
        Vec::new()
    }
}

fn clamp_u8(value: i64, max: u8) -> u8 {
    u8::try_from(value.clamp(0, i64::from(max))).unwrap_or(0)
}

fn character_map(
    file: &GvasFile,
) -> Result<impl Iterator<Item = (&Property, &Property)>> {
    let world = custom_struct(file.properties.0.get("worldSaveData"))
        .ok_or_else(|| PalworldError::Gvas("missing worldSaveData struct".into()))?;
    let cspm = world
        .0
        .get("CharacterSaveParameterMap")
        .and_then(|v| v.first())
        .ok_or_else(|| {
            PalworldError::Gvas("missing CharacterSaveParameterMap".into())
        })?;
    let Property::MapProperty(MapProperty::Properties { value, .. }) = cspm else {
        return Err(PalworldError::Gvas(
            "CharacterSaveParameterMap has unexpected shape".into(),
        ));
    };
    Ok(value.0.iter())
}

fn rawdata(val: &Property) -> Option<&[u8]> {
    let fields = struct_fields(val)?;
    if let Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) =
        field(fields, "RawData")
    {
        Some(bytes)
    } else {
        None
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveEdits {
    pub player_edits: Vec<PlayerEdit>,
    pub pal_edits: Vec<PalEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerEdit {
    pub instance_id: String,
    pub level: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PalEdit {
    pub instance_id: String,
    pub level: Option<i32>,
    pub talent_hp: Option<u8>,
    pub talent_shot: Option<u8>,
    pub talent_defense: Option<u8>,
    pub traits: Option<Vec<String>>,
}

#[derive(Default)]
struct Change {
    level: Option<i32>,
    talent_hp: Option<u8>,
    talent_shot: Option<u8>,
    talent_defense: Option<u8>,
    traits: Option<Vec<String>>,
}

pub const STATUS_POINTS_PER_LEVEL: i32 = 1;

#[derive(Debug, Clone)]
pub struct EditedSave {
    pub level: Vec<u8>,
    pub level_deltas: Vec<(String, i32)>,
}

pub fn apply_edits(level_bytes: &[u8], edits: &SaveEdits) -> Result<EditedSave> {
    let ty = compress::source_type_byte(level_bytes)?;
    let decompressed = decompress::decompress(level_bytes)?;
    let mut file = save_gvas::read_gvas(&decompressed)?;
    let custom_versions = file.header.get_custom_versions().clone();

    let mut pending = pending_changes(edits);
    let level_deltas = patch_characters(&mut file, &custom_versions, &mut pending)?;

    if let Some(missing) = pending.keys().next() {
        return Err(PalworldError::Edit(format!(
            "no character with instance id {missing} in this save - the world \
             may have been refreshed since the roster was loaded"
        )));
    }

    let mut out = Cursor::new(Vec::new());
    file.write(&mut out).map_err(|e| PalworldError::Gvas(e.to_string()))?;
    Ok(EditedSave {
        level: compress::compress(&out.into_inner(), ty)?,
        level_deltas,
    })
}

fn pending_changes(edits: &SaveEdits) -> HashMap<String, Change> {
    let players = edits.player_edits.iter().map(|e| {
        (e.instance_id.clone(), Change { level: e.level, ..Change::default() })
    });
    let pals = edits.pal_edits.iter().map(|e| {
        (e.instance_id.clone(), Change {
            level: e.level,
            talent_hp: e.talent_hp,
            talent_shot: e.talent_shot,
            talent_defense: e.talent_defense,
            traits: e.traits.clone(),
        })
    });
    players.chain(pals).collect()
}

fn patch_characters(
    file: &mut GvasFile,
    custom_versions: &HashableIndexMap<Guid, u32>,
    pending: &mut HashMap<String, Change>,
) -> Result<Vec<(String, i32)>> {
    let mut deltas = Vec::new();
    if pending.is_empty() {
        return Ok(deltas);
    }

    for (key, val) in character_map_mut(file)? {
        let Some(instance_id) = key_instance_id(key) else { continue };
        let Some(change) = pending.remove(&instance_id) else { continue };
        let player_uid = key_player_uid(key);

        let raw = rawdata_mut(val).ok_or_else(|| {
            PalworldError::Edit(format!("character {instance_id} has no RawData"))
        })?;
        let mut parsed = save_gvas::reparse_properties_at(raw, custom_versions)?;

        let save_param = parsed
            .properties
            .iter_mut()
            .find(|(k, _)| k == "SaveParameter")
            .and_then(|(_, p)| struct_fields_mut(p))
            .ok_or_else(|| {
                PalworldError::Edit(format!(
                    "character {instance_id} has no SaveParameter"
                ))
            })?;

        let delta = apply_change(save_param, &change);
        if let Some(uid) = player_uid
            && delta != 0
            && bool_field(save_param, "IsPlayer")
        {
            deltas.push((uid, delta));
        }
        *raw = save_gvas::write_properties(&parsed, custom_versions)?;
    }

    Ok(deltas)
}

/// Applies one character's changes and reports the level delta, which is zero
/// unless the level actually moved.
fn apply_change(
    save_param: &mut HashableIndexMap<String, Vec<Property>>,
    change: &Change,
) -> i32 {
    let mut delta = 0;
    if let Some(level) = change.level {
        let level = level.clamp(1, 255);
        let old = i32::try_from(int_field(save_param, "Level")).unwrap_or(1).max(1);
        delta = level - old;
        let level = u8::try_from(level).unwrap_or(1);
        set(save_param, "Level", byte(level));
        set(save_param, "Exp", Property::Int64Property(Int64Property::new(0)));
        grant_status_points(save_param, delta);
    }
    for (name, value) in [
        ("Talent_HP", change.talent_hp),
        ("Talent_Shot", change.talent_shot),
        ("Talent_Defense", change.talent_defense),
    ] {
        if let Some(value) = value {
            set(save_param, name, byte(value.min(100)));
        }
    }
    if let Some(traits) = &change.traits {
        let names = traits.iter().cloned().map(Some).collect();
        set(
            save_param,
            "PassiveSkillList",
            Property::ArrayProperty(ArrayProperty::Names { names }),
        );
    }
    delta
}

fn grant_status_points(
    save_param: &mut HashableIndexMap<String, Vec<Property>>,
    delta: i32,
) {
    if !bool_field(save_param, "IsPlayer") {
        return;
    }
    let current = if let Some(Property::UInt16Property(p)) =
        field(save_param, "UnusedStatusPoint")
    {
        i32::from(p.value)
    } else {
        0
    };
    let granted = delta.saturating_mul(STATUS_POINTS_PER_LEVEL);
    let next =
        u16::try_from(current.saturating_add(granted).max(0)).unwrap_or(u16::MAX);
    set(
        save_param,
        "UnusedStatusPoint",
        Property::UInt16Property(UInt16Property::new(next)),
    );
}

fn set(
    save_param: &mut HashableIndexMap<String, Vec<Property>>,
    name: &str,
    property: Property,
) {
    match save_param.0.get_mut(name).and_then(|v| v.first_mut()) {
        Some(existing) => *existing = property,
        None => {
            let _ = save_param.0.insert(name.to_string(), vec![property]);
        },
    }
}

fn byte(value: u8) -> Property {
    Property::ByteProperty(ByteProperty::new(
        Some("None".to_string()),
        BytePropertyValue::Byte(value),
    ))
}

const fn struct_fields_mut(
    prop: &mut Property,
) -> Option<&mut HashableIndexMap<String, Vec<Property>>> {
    let value = if let Property::StructProperty(s) = prop {
        &mut s.value
    } else if let Property::StructPropertyValue(v) = prop {
        v
    } else {
        return None;
    };
    if let StructPropertyValue::CustomStruct(m) = value { Some(m) } else { None }
}

fn character_map_mut(
    file: &mut GvasFile,
) -> Result<impl Iterator<Item = (&Property, &mut Property)>> {
    let world = file
        .properties
        .0
        .get_mut("worldSaveData")
        .and_then(struct_fields_mut)
        .ok_or_else(|| PalworldError::Gvas("missing worldSaveData struct".into()))?;
    let cspm = world
        .0
        .get_mut("CharacterSaveParameterMap")
        .and_then(|v| v.first_mut())
        .ok_or_else(|| {
            PalworldError::Gvas("missing CharacterSaveParameterMap".into())
        })?;
    let Property::MapProperty(MapProperty::Properties { value, .. }) = cspm else {
        return Err(PalworldError::Gvas(
            "CharacterSaveParameterMap has unexpected shape".into(),
        ));
    };
    Ok(value.0.iter_mut())
}

fn rawdata_mut(val: &mut Property) -> Option<&mut Vec<u8>> {
    let fields = struct_fields_mut(val)?;
    if let Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) =
        fields.0.get_mut("RawData").and_then(|v| v.first_mut())
    {
        Some(bytes)
    } else {
        None
    }
}
