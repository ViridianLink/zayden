use std::collections::HashMap;

use gvas::GvasFile;
use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;

use super::extract::{custom_struct, field, hex_upper, struct_fields};

const GUILD_TYPE: &str = "EPalGroupType::Guild";
const HANDLE_SIZE: usize = 32;
const MAX_MEMBERS: i32 = 64;
const MAX_NAME_LEN: i32 = 64;

#[derive(Debug, Default)]
pub struct GuildData {
    player_guild: HashMap<String, String>,
    guild_members: HashMap<String, Vec<String>>,
}

impl GuildData {
    #[must_use]
    pub fn guild_of(&self, uid: &str) -> Option<&str> {
        self.player_guild.get(uid).map(String::as_str)
    }

    #[must_use]
    pub fn members(&self, guild_id: &str) -> &[String] {
        self.guild_members.get(guild_id).map_or(&[], Vec::as_slice)
    }

    pub fn all_members(&self) -> impl Iterator<Item = &String> {
        self.guild_members.values().flatten()
    }
}

#[must_use]
pub fn decode_guilds(level: &GvasFile) -> GuildData {
    let mut data = GuildData::default();

    let Some(world) = custom_struct(level.properties.0.get("worldSaveData")) else {
        return data;
    };
    let Some(Property::MapProperty(MapProperty::Properties { value, .. })) =
        world.0.get("GroupSaveDataMap").and_then(|v| v.first())
    else {
        return data;
    };

    for (_key, val) in &value.0 {
        let Some(fields) = struct_fields(val) else { continue };

        let is_guild = matches!(
            field(fields, "GroupType"),
            Some(Property::EnumProperty(e)) if e.value == GUILD_TYPE
        );
        if !is_guild {
            continue;
        }

        let Some(Property::ArrayProperty(ArrayProperty::Bytes { bytes })) =
            field(fields, "RawData")
        else {
            continue;
        };

        let Some(guild_id) = bytes.get(0..16).map(hex_upper) else { continue };
        let Some(start) = tail_start(bytes) else { continue };
        let members = locate_members(bytes, start);
        if members.is_empty() {
            continue;
        }

        for uid in &members {
            data.player_guild.insert(uid.clone(), guild_id.clone());
        }
        data.guild_members.insert(guild_id, members);
    }

    data
}

fn tail_start(bytes: &[u8]) -> Option<usize> {
    let mut pos = 16usize; // group_id
    let name_len = read_i32(bytes, &mut pos)?;
    if let Ok(name_len) = usize::try_from(name_len) {
        pos = pos.checked_add(name_len)?;
    }
    let handles = read_i32(bytes, &mut pos)?;
    let handles = usize::try_from(handles).ok()?;
    pos = pos.checked_add(handles.checked_mul(HANDLE_SIZE)?)?;
    (pos <= bytes.len()).then_some(pos)
}

fn locate_members(bytes: &[u8], start: usize) -> Vec<String> {
    let mut best: Vec<String> = Vec::new();
    let mut p = start;
    while p + 4 <= bytes.len() {
        if let Some(members) = try_members(bytes, p)
            && members.len() >= best.len()
        {
            best = members;
        }
        p += 1;
    }
    best
}

fn try_members(bytes: &[u8], p: usize) -> Option<Vec<String>> {
    let mut pos = p;
    let count = read_i32(bytes, &mut pos)?;
    if !(1..=MAX_MEMBERS).contains(&count) {
        return None;
    }

    let mut uids = Vec::new();
    for _ in 0..count {
        let uid = read_guid(bytes, &mut pos)?;
        pos = pos.checked_add(8)?; // last_online i64
        skip_fstring(bytes, &mut pos)?; // player name
        pos = pos.checked_add(1)?; // role byte
        if pos > bytes.len() {
            return None;
        }
        uids.push(uid);
    }
    Some(uids)
}

fn read_i32(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    let arr: [u8; 4] = bytes.get(*pos..pos.checked_add(4)?)?.try_into().ok()?;
    *pos += 4;
    Some(i32::from_le_bytes(arr))
}

fn read_guid(bytes: &[u8], pos: &mut usize) -> Option<String> {
    let end = pos.checked_add(16)?;
    let hex = hex_upper(bytes.get(*pos..end)?);
    *pos = end;
    Some(hex)
}

fn skip_fstring(bytes: &[u8], pos: &mut usize) -> Option<()> {
    let len = read_i32(bytes, pos)?;
    if !(1..=MAX_NAME_LEN).contains(&len) {
        return None;
    }
    let len = usize::try_from(len).ok()?;
    let end = pos.checked_add(len)?;
    let body = bytes.get(*pos..end.checked_sub(1)?)?;
    if bytes.get(end - 1) != Some(&0) {
        return None;
    }
    if !body.iter().all(|&b| b.is_ascii_graphic() || b == b' ') {
        return None;
    }
    *pos = end;
    Some(())
}
