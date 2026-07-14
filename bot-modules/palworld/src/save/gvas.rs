use std::collections::HashMap;
use std::io::Cursor;

use gvas::GvasFile;
use gvas::cursor_ext::ReadExt;
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
    ];

    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

pub fn read_gvas(bytes: &[u8]) -> Result<GvasFile> {
    GvasFile::read_with_hints(
        &mut Cursor::new(bytes),
        GameVersion::Default,
        &hints(),
    )
    .map_err(|e| PalworldError::Gvas(e.to_string()))
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
