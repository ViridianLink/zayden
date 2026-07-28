use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use gvas::properties::Property;
use gvas::properties::array_property::ArrayProperty;
use gvas::properties::map_property::MapProperty;
use gvas::properties::struct_property::StructPropertyValue;
use gvas::types::map::HashableIndexMap;

use super::decompress;
use super::extract::{bool_field, field, int_field, struct_fields};
use crate::error::{PalworldError, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerRecord {
    pub uid: String,

    pub fast_travel: BTreeSet<String>,
    pub areas_found: BTreeSet<String>,
    pub effigies: BTreeSet<String>,
    pub relics_by_type: BTreeMap<String, BTreeSet<String>>,
    pub relics_unspent: BTreeMap<String, i64>,
    pub bosses_defeated: BTreeSet<String>,
    pub towers_defeated: BTreeSet<String>,
    pub paldeck_seen: BTreeSet<String>,
    pub pal_captures: BTreeMap<String, i64>,
    pub pal_capture_bonus: BTreeMap<String, i64>,
    pub technologies: BTreeSet<String>,
    pub quests_completed: BTreeSet<String>,
    pub quests_active: BTreeSet<String>,

    pub notes: BTreeSet<String>,
    pub item_pickups: BTreeSet<String>,
    pub npcs_talked: BTreeMap<String, i64>,
    pub npc_rewards: BTreeSet<String>,
    pub crafted_items: BTreeMap<String, i64>,
    pub fishing_counts: BTreeMap<String, i64>,
    pub raid_boss_defeats: BTreeMap<String, i64>,
    pub pal_rankups: BTreeMap<String, i64>,
    pub world_maps: BTreeSet<String>,
    pub area_barriers: BTreeSet<String>,
    pub emote_npcs: BTreeSet<String>,
    pub treasure_points: BTreeSet<String>,
    pub arena_ranks: BTreeMap<String, i64>,

    pub tribe_captures: i64,
    pub normal_dungeons_cleared: i64,
    pub fixed_dungeons_cleared: i64,
    pub oilrigs_cleared: i64,
    pub camps_conquered: i64,
    pub treasures_found: i64,
    pub predators_defeated: i64,
    pub awakenings: i64,
    pub mutations: i64,
    pub technology_points: i64,
    pub boss_technology_points: i64,
    pub game_cleared: bool,
    pub first_fishing_done: bool,

    pub exp_bonus_tiers: BTreeMap<&'static str, i64>,
}

impl PlayerRecord {
    #[must_use]
    pub fn treasures(&self) -> i64 {
        let points = i64::try_from(self.treasure_points.len()).unwrap_or(i64::MAX);
        self.treasures_found.max(points)
    }
}

pub fn load_player(save_dir: &Path, uid: &str) -> Result<Option<PlayerRecord>> {
    let Some(path) = super::player_save_path(save_dir, uid) else {
        return Ok(None);
    };
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    parse_player(&raw, uid).map(Some)
}

pub fn parse_player(raw: &[u8], uid: &str) -> Result<PlayerRecord> {
    let file = super::gvas::read_gvas(&decompress::decompress(raw)?)?;
    let save_data = file
        .properties
        .0
        .get("SaveData")
        .and_then(struct_fields)
        .ok_or_else(|| {
            PalworldError::Gvas("not a player save: missing SaveData struct".into())
        })?;

    let record = field(save_data, "RecordData").and_then(struct_fields);
    let empty = HashableIndexMap::default();
    let record = record.unwrap_or(&empty);

    Ok(PlayerRecord {
        uid: uid.to_string(),

        fast_travel: flag_keys(record, "FastTravelPointUnlockFlag"),
        areas_found: flag_keys(record, "FindAreaFlagMap"),
        effigies: flag_keys(record, "RelicObtainForInstanceFlag"),
        relics_by_type: relics_by_type(record),
        relics_unspent: relic_wallet(record),
        bosses_defeated: flag_keys(record, "NormalBossDefeatFlag"),
        towers_defeated: flag_keys(record, "TowerBossDefeatFlag"),
        paldeck_seen: flag_keys(record, "PaldeckUnlockFlag"),
        pal_captures: count_map(record, "PalCaptureCount"),
        pal_capture_bonus: count_map(record, "PalCaptureBonusCount"),
        technologies: name_array(save_data, "UnlockedRecipeTechnologyNames"),
        quests_completed: quest_array(save_data, "CompletedQuestArray"),
        quests_active: quest_array(save_data, "OrderedQuestArray"),

        notes: flag_keys(record, "NoteObtainForInstanceFlag"),
        item_pickups: flag_keys(record, "ItemPickupObtainForInstanceFlag"),
        npcs_talked: count_map(record, "NPCTalkCountMap"),
        npc_rewards: flag_keys(record, "NPCAchivementRewardFlag"),
        crafted_items: count_map(record, "CraftItemCount"),
        fishing_counts: count_map(record, "FishingCountMap"),
        raid_boss_defeats: count_map(record, "RaidBossDefeatCount"),
        pal_rankups: count_map(record, "PalRankupCount"),
        world_maps: flag_keys(record, "UnlockedWorldMapFlags"),
        area_barriers: flag_keys(record, "AreaBarrierUnlockFlags"),
        emote_npcs: name_array(record, "CompletedEmoteNPCIDArray"),
        treasure_points: map_keys(record, "FoundTreasureMapPointMap"),
        arena_ranks: count_map(record, "ArenaSoloClearCount"),

        tribe_captures: int_field(record, "TribeCaptureCount"),
        normal_dungeons_cleared: int_field(record, "NormalDungeonClearCount"),
        fixed_dungeons_cleared: int_field(record, "FixedDungeonClearCount"),
        oilrigs_cleared: int_field(record, "OilrigClearCount"),
        camps_conquered: int_field(record, "CampConqueredCount"),
        treasures_found: int_field(record, "FoundTreasureCount"),
        predators_defeated: int_field(record, "PredatorDefeatCount"),
        awakenings: int_field(record, "AwakeningCount"),
        mutations: int_field(record, "MutationCount"),
        technology_points: int_field(save_data, "TechnologyPoint"),
        boss_technology_points: int_field(save_data, "bossTechnologyPoint"),
        game_cleared: bool_field(record, "bIsGameCleared"),
        first_fishing_done: bool_field(record, "bFirstFishingComplete"),

        exp_bonus_tiers: exp_bonus_tiers(record),
    })
}

pub fn parse_player_uid(raw: &[u8]) -> Result<String> {
    let file = super::gvas::read_gvas(&decompress::decompress(raw)?)?;
    file.properties
        .0
        .get("SaveData")
        .and_then(struct_fields)
        .and_then(|save_data| field(save_data, "PlayerUId"))
        .and_then(guid_hex)
        .ok_or_else(|| {
            PalworldError::Gvas(
                "not a player save: missing SaveData.PlayerUId".into(),
            )
        })
}

const EXP_BONUSES: &[(&str, &str)] = &[
    ("Area", "AreaBonusExpTableIndex"),
    ("Boss", "BossDefeatExpBonusTableIndex"),
    ("Capture", "PalCaptureBonusExpTableIndex"),
    ("Fast travel", "FastTravelBonusExpTableIndex"),
    ("Item pickup", "ItemPIckupBonusExpTableIndex"),
    ("Note", "NoteBonusExpTableIndex"),
    ("NPC", "NpcBonusExpTableIndex"),
    ("Relic", "RelicBonusExpTableIndex"),
];

fn exp_bonus_tiers(
    record: &HashableIndexMap<String, Vec<Property>>,
) -> BTreeMap<&'static str, i64> {
    EXP_BONUSES
        .iter()
        .filter_map(|(label, key)| {
            let value = int_field(record, key);
            (value > 0).then_some((*label, value))
        })
        .collect()
}

fn flag_keys(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> BTreeSet<String> {
    let Some(Property::MapProperty(map)) = field(fields, name) else {
        return BTreeSet::new();
    };

    let set = |pairs: &HashableIndexMap<String, bool>| {
        pairs.0.iter().filter(|(_, v)| **v).map(|(k, _)| k.clone()).collect()
    };

    match map {
        MapProperty::NameBool { name_bools } => set(name_bools),
        MapProperty::StrBool { str_bools } => set(str_bools),
        MapProperty::EnumBool { enum_bools } => set(enum_bools),
        MapProperty::NameProperty { name_props, .. } => {
            truthy(name_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::StrProperty { str_props, .. } => {
            truthy(str_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::EnumProperty { enum_props, .. } => {
            truthy(enum_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::Properties { value, .. } => {
            truthy(value.0.iter().filter_map(|(k, v)| Some((key_string(k)?, v))))
        },
        // Numeric and string-valued maps carry no flags.
        MapProperty::EnumInt { .. }
        | MapProperty::NameInt { .. }
        | MapProperty::StrInt { .. }
        | MapProperty::StrStr { .. } => BTreeSet::new(),
    }
}

fn truthy<'a>(
    pairs: impl Iterator<Item = (String, &'a Property)>,
) -> BTreeSet<String> {
    pairs
        .filter(|(_, v)| matches!(v, Property::BoolProperty(b) if b.value))
        .map(|(k, _)| k)
        .collect()
}

fn map_keys(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> BTreeSet<String> {
    fn keys<V: std::hash::Hash>(
        pairs: &HashableIndexMap<String, V>,
    ) -> BTreeSet<String> {
        pairs.0.keys().cloned().collect()
    }

    let Some(Property::MapProperty(map)) = field(fields, name) else {
        return BTreeSet::new();
    };

    match map {
        MapProperty::NameBool { name_bools } => keys(name_bools),
        MapProperty::StrBool { str_bools } => keys(str_bools),
        MapProperty::EnumBool { enum_bools } => keys(enum_bools),
        MapProperty::NameInt { name_ints } => keys(name_ints),
        MapProperty::StrInt { str_ints } => keys(str_ints),
        MapProperty::EnumInt { enum_ints } => keys(enum_ints),
        MapProperty::StrStr { str_strs } => keys(str_strs),
        MapProperty::NameProperty { name_props, .. } => keys(name_props),
        MapProperty::StrProperty { str_props, .. } => keys(str_props),
        MapProperty::EnumProperty { enum_props, .. } => keys(enum_props),
        MapProperty::Properties { value, .. } => {
            value.0.keys().filter_map(key_string).collect()
        },
    }
}

fn count_map(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> BTreeMap<String, i64> {
    let Some(Property::MapProperty(map)) = field(fields, name) else {
        return BTreeMap::new();
    };

    let ints = |pairs: &HashableIndexMap<String, i32>| {
        pairs
            .0
            .iter()
            .filter(|(_, v)| **v > 0)
            .map(|(k, v)| (k.clone(), i64::from(*v)))
            .collect()
    };

    match map {
        MapProperty::NameInt { name_ints } => ints(name_ints),
        MapProperty::StrInt { str_ints } => ints(str_ints),
        MapProperty::EnumInt { enum_ints } => ints(enum_ints),
        MapProperty::NameProperty { name_props, .. } => {
            counted(name_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::StrProperty { str_props, .. } => {
            counted(str_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::EnumProperty { enum_props, .. } => {
            counted(enum_props.0.iter().map(|(k, v)| (k.clone(), v)))
        },
        MapProperty::Properties { value, .. } => {
            counted(value.0.iter().filter_map(|(k, v)| Some((key_string(k)?, v))))
        },
        MapProperty::EnumBool { .. }
        | MapProperty::NameBool { .. }
        | MapProperty::StrBool { .. }
        | MapProperty::StrStr { .. } => BTreeMap::new(),
    }
}

fn counted<'a>(
    pairs: impl Iterator<Item = (String, &'a Property)>,
) -> BTreeMap<String, i64> {
    pairs
        .filter_map(|(k, v)| {
            let count = if let Property::IntProperty(i) = v {
                i64::from(i.value)
            } else if let Property::Int64Property(i) = v {
                i.value
            } else {
                return None;
            };
            (count > 0).then_some((k, count))
        })
        .collect()
}

fn relics_by_type(
    record: &HashableIndexMap<String, Vec<Property>>,
) -> BTreeMap<String, BTreeSet<String>> {
    let entries = match field(record, "RelicObtainForInstanceFlagByType") {
        Some(Property::ArrayProperty(ArrayProperty::Structs {
            structs, ..
        })) => structs.iter().filter_map(custom_struct_fields).collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    entries
        .into_iter()
        .filter_map(|fields| {
            let Property::EnumProperty(kind) = field(fields, "Type")? else {
                return None;
            };
            let ids = flag_keys(fields, "Flags");
            Some((relic_type_key(&kind.value), ids))
        })
        .filter(|(_, ids)| !ids.is_empty())
        .collect()
}

fn relic_wallet(
    record: &HashableIndexMap<String, Vec<Property>>,
) -> BTreeMap<String, i64> {
    count_map(record, "RelicPossessNumMap")
        .into_iter()
        .map(|(k, v)| (relic_type_key(&k), v))
        .collect()
}

fn relic_type_key(raw: &str) -> String {
    pascal_to_snake(raw.rsplit("::").next().unwrap_or(raw))
}

fn pascal_to_snake(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    for (i, ch) in raw.char_indices() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn key_string(key: &Property) -> Option<String> {
    if let Property::NameProperty(n) = key {
        n.value.clone()
    } else if let Property::StrProperty(s) = key {
        s.value.clone()
    } else {
        guid_hex(key)
    }
}

fn guid_hex(prop: &Property) -> Option<String> {
    let value = if let Property::StructProperty(s) = prop {
        &s.value
    } else if let Property::StructPropertyValue(v) = prop {
        v
    } else {
        return None;
    };
    if let StructPropertyValue::Guid(guid) = value {
        Some(super::extract::hex_upper(&guid.0))
    } else {
        None
    }
}

fn name_array(
    fields: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> BTreeSet<String> {
    match field(fields, name) {
        Some(Property::ArrayProperty(ArrayProperty::Names { names })) => {
            names.iter().flatten().cloned().collect()
        },
        Some(Property::ArrayProperty(ArrayProperty::Strings { strings })) => {
            strings.iter().flatten().cloned().collect()
        },
        Some(Property::ArrayProperty(ArrayProperty::Properties {
            properties,
            ..
        })) => properties.iter().filter_map(key_string).collect(),
        _ => BTreeSet::new(),
    }
}

fn quest_array(
    save_data: &HashableIndexMap<String, Vec<Property>>,
    base: &str,
) -> BTreeSet<String> {
    let full = format!("{base}_FullRelease");
    let mut ids = quest_ids(save_data, &full);
    if ids.is_empty() {
        ids = quest_ids(save_data, base);
    }
    ids
}

fn quest_ids(
    save_data: &HashableIndexMap<String, Vec<Property>>,
    name: &str,
) -> BTreeSet<String> {
    match field(save_data, name) {
        Some(Property::ArrayProperty(ArrayProperty::Names { names })) => {
            names.iter().flatten().cloned().collect()
        },
        Some(Property::ArrayProperty(ArrayProperty::Properties {
            properties,
            ..
        })) => properties.iter().filter_map(quest_id).collect(),
        Some(Property::ArrayProperty(ArrayProperty::Structs {
            structs, ..
        })) => structs.iter().filter_map(struct_quest_id).collect(),
        _ => BTreeSet::new(),
    }
}

fn quest_id(prop: &Property) -> Option<String> {
    if let Some(id) = key_string(prop) {
        return Some(id);
    }
    struct_fields(prop).and_then(quest_id_from_fields)
}

fn struct_quest_id(value: &StructPropertyValue) -> Option<String> {
    custom_struct_fields(value).and_then(quest_id_from_fields)
}

const fn custom_struct_fields(
    value: &StructPropertyValue,
) -> Option<&HashableIndexMap<String, Vec<Property>>> {
    if let StructPropertyValue::CustomStruct(fields) = value {
        Some(fields)
    } else {
        None
    }
}

fn quest_id_from_fields(
    fields: &HashableIndexMap<String, Vec<Property>>,
) -> Option<String> {
    for name in ["QuestID", "QuestId", "QuestName"] {
        if let Some(id) = field(fields, name).and_then(key_string) {
            return Some(id);
        }
    }
    fields.0.values().filter_map(|v| v.first()).find_map(key_string)
}

pub fn unknown_record_keys(raw: &[u8]) -> Result<Vec<String>> {
    const KNOWN: &[&str] = &[
        "AreaBarrierUnlockFlags",
        "AreaBonusExpTableIndex",
        "ArenaSoloClearCount",
        "AwakeningCount",
        "BossDefeatExpBonusTableIndex",
        "BuildingObjectMapObjectInstanceIds",
        "CampConqueredCount",
        "CompletedEmoteNPCIDArray",
        "CraftItemCount",
        "FastTravelBonusExpTableIndex",
        "FastTravelPointUnlockFlag",
        "FindAreaFlagMap",
        "FishingCountMap",
        "FixedDungeonClearCount",
        "FoundTreasureCount",
        "FoundTreasureMapPointMap",
        "InvokeNPCNetworkEventMap",
        "ItemPIckupBonusExpTableIndex",
        "ItemPickupObtainForInstanceFlag",
        "MutationCount",
        "NPCAchivementRewardFlag",
        "NPCTalkCountMap",
        "NormalBossDefeatFlag",
        "NormalDungeonClearCount",
        "NoteBonusExpTableIndex",
        "NoteObtainForInstanceFlag",
        "NpcBonusExpTableIndex",
        "OilrigClearCount",
        "PalCaptureBonusCount",
        "PalCaptureBonusExpTableIndex",
        "PalCaptureCount",
        "PalRankupCount",
        "PaldeckUnlockFlag",
        "PredatorDefeatCount",
        "RaidBossDefeatCount",
        "RelicBonusExpTableIndex",
        "RelicObtainForInstanceFlag",
        "RelicObtainForInstanceFlagByType",
        "RelicPossessNum",
        "RelicPossessNumMap",
        "TowerBossDefeatCount",
        "TowerBossDefeatFlag",
        "TribeCaptureCount",
        "UnlockedWorldMapFlags",
        "bCaptureCompletionRelicFixupDone",
        "bFieldBossDefeatFlagResetDone",
        "bFirstFishingComplete",
        "bIsGameCleared",
    ];

    let file = super::gvas::read_gvas(&decompress::decompress(raw)?)?;
    let Some(record) = file
        .properties
        .0
        .get("SaveData")
        .and_then(struct_fields)
        .and_then(|sd| field(sd, "RecordData"))
        .and_then(struct_fields)
    else {
        return Ok(Vec::new());
    };

    Ok(record.0.keys().filter(|k| !KNOWN.contains(&k.as_str())).cloned().collect())
}
