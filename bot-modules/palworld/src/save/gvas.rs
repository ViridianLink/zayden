use std::collections::HashMap;
use std::io::Cursor;

use gvas::GvasFile;
use gvas::cursor_ext::ReadExt;
use gvas::error::{DeserializeError, Error};
use gvas::game_version::GameVersion;
use gvas::properties::{Property, PropertyOptions};
use gvas::types::Guid;
use gvas::types::map::HashableIndexMap;

use crate::error::{PalworldError, Result};

#[must_use]
pub fn hints() -> HashMap<String, String> {
    const S: &str = "StructProperty";
    const G: &str = "Guid";
    let pairs: &[(&str, &str)] = &[
        (
            "worldSaveData.StructProperty.CharacterSaveParameterMap.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.CharacterSaveParameterMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSaveData.ArrayProperty.ConcreteModel.StructProperty.ModuleMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSaveData.ArrayProperty.Model.StructProperty.EffectMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FoliageGridSaveDataMap.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FoliageGridSaveDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FoliageGridSaveDataMap.MapProperty.Value.StructProperty.ModelMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FoliageGridSaveDataMap.MapProperty.Value.StructProperty.ModelMap.MapProperty.Value.StructProperty.InstanceDataMap.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FoliageGridSaveDataMap.MapProperty.Value.StructProperty.ModelMap.MapProperty.Value.StructProperty.InstanceDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSpawnerInStageSaveData.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSpawnerInStageSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSpawnerInStageSaveData.MapProperty.Value.StructProperty.SpawnerDataMapByLevelObjectInstanceId.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSpawnerInStageSaveData.MapProperty.Value.StructProperty.SpawnerDataMapByLevelObjectInstanceId.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.MapObjectSpawnerInStageSaveData.MapProperty.Value.StructProperty.SpawnerDataMapByLevelObjectInstanceId.MapProperty.Value.StructProperty.ItemMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.BaseCampSaveData.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.BaseCampSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.BaseCampSaveData.MapProperty.Value.StructProperty.ModuleMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.ItemContainerSaveData.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.ItemContainerSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.CharacterContainerSaveData.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.CharacterContainerSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.GroupSaveDataMap.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.GroupSaveDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.EnemyCampSaveData.StructProperty.EnemyCampStatusMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.EnemyCampSaveData.StructProperty.EnemyCampStatusMap.MapProperty.Value.StructProperty.TreasureBoxInfoMapBySpawnerName.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.LockGimmickSaveData.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.LockGimmickSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.WorkSaveData.ArrayProperty.WorkAssignMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.GuildExtraSaveDataMap.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.GuildExtraSaveDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.DungeonSaveData.ArrayProperty.MapObjectSaveData.ArrayProperty.ConcreteModel.StructProperty.ModuleMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.InvaderSaveData.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.InvaderSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.OilrigSaveData.StructProperty.OilrigMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.InLockerCharacterInstanceIDArray.SetProperty.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.DungeonSaveData.ArrayProperty.RewardSaveDataMap.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.DungeonSaveData.ArrayProperty.RewardSaveDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.FishingSpotSaveData.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.FishingSpotSaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.LevelObjectRecoverPartySaveData.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.LevelObjectRecoverPartySaveData.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.LevelObjectRecoverPartySaveData.MapProperty.Value.StructProperty.PlayerLastUsedTimes.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.SupplySaveData.StructProperty.SupplyInfos.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.SupplySaveData.StructProperty.SupplyInfos.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.RaidBossAreaInstanceSaveDataMap.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "worldSaveData.StructProperty.RaidBossAreaInstanceSaveDataMap.MapProperty.Value.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.RaidBossAreaInstanceSaveDataMap.MapProperty.Value.StructProperty.BaseCampWorkerSpawnedByPlayerMap.MapProperty.Key.StructProperty",
            S,
        ),
        (
            "worldSaveData.StructProperty.RaidBossAreaInstanceSaveDataMap.MapProperty.Value.StructProperty.BaseCampWorkerSpawnedByPlayerMap.MapProperty.Value.StructProperty",
            G,
        ),
        (
            "SaveData.StructProperty.RecordData.StructProperty.FoundTreasureMapPointMap.MapProperty.Key.StructProperty",
            G,
        ),
        (
            "SaveData.StructProperty.RecordData.StructProperty.FoundTreasureMapPointMap.MapProperty.Value.StructProperty",
            S,
        ),
    ];

    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

const CANDIDATES: [&str; 2] = ["Guid", "StructProperty"];

const FIRST_CANDIDATE: &str = "Guid";

const MAX_INFERRED: usize = 8;

pub fn read_gvas(bytes: &[u8]) -> Result<GvasFile> {
    read_inferring(bytes, hints())
}

pub fn read_inferring(
    bytes: &[u8],
    base: impl IntoIterator<Item = (String, String)>,
) -> Result<GvasFile> {
    let mut hints: HashMap<String, String> = base.into_iter().collect();
    let mut inferred: Vec<(String, usize)> = Vec::new();

    loop {
        match read_with(bytes, &hints) {
            Ok(file) => {
                if !inferred.is_empty() {
                    tracing::warn!(
                        hints = ?inferred
                            .iter()
                            .filter_map(|(p, i)| {
                                Some(format!("{p} = {}", CANDIDATES.get(*i)?))
                            })
                            .collect::<Vec<_>>(),
                        "palworld: save contains structs missing from the hint \
                         table; inferred them for this read. Add them to \
                         save::gvas::hints so the first parse succeeds.",
                    );
                }
                return Ok(file);
            },
            Err(Error::Deserialize(DeserializeError::MissingHint(_, path, _)))
                if inferred.len() < MAX_INFERRED
                    && !hints.contains_key(path.as_ref()) =>
            {
                let path = path.into_string();
                hints.insert(path.clone(), FIRST_CANDIDATE.to_string());
                inferred.push((path, 0));
            },
            Err(e) => {
                if !backtrack(&mut hints, &mut inferred) {
                    return Err(PalworldError::Gvas(e.to_string()));
                }
            },
        }
    }
}

fn backtrack(
    hints: &mut HashMap<String, String>,
    inferred: &mut Vec<(String, usize)>,
) -> bool {
    while let Some((path, index)) = inferred.last_mut() {
        if let Some(next) = CANDIDATES.get(*index + 1) {
            *index += 1;
            hints.insert(path.clone(), (*next).to_string());
            return true;
        }
        hints.remove(path);
        inferred.pop();
    }
    false
}

fn read_with(
    bytes: &[u8],
    hints: &HashMap<String, String>,
) -> std::result::Result<GvasFile, Error> {
    GvasFile::read_with_hints(&mut Cursor::new(bytes), GameVersion::Default, hints)
}

pub fn reparse_properties(
    bytes: &[u8],
    custom_versions: &HashableIndexMap<Guid, u32>,
) -> Result<Vec<(String, Property)>> {
    let mut cursor = Cursor::new(bytes);
    let mut stack: Vec<String> = Vec::new();
    let hints = HashMap::new();
    let mut options = PropertyOptions {
        hints: &hints,
        properties_stack: &mut stack,
        custom_versions,
    };

    let mut out = Vec::new();
    while let Ok(name) = cursor.read_string() {
        if name == "None" {
            break;
        }
        let ty =
            cursor.read_string().map_err(|e| PalworldError::Gvas(e.to_string()))?;

        options.properties_stack.push(name.clone());
        let property = Property::new(&mut cursor, &ty, true, &mut options, None)
            .map_err(|e| PalworldError::Gvas(e.to_string()));
        let _ = options.properties_stack.pop();

        out.push((name, property?));
    }

    Ok(out)
}
