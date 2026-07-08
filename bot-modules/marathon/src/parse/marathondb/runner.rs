use serde_json::Value;

use super::{current_or_last_stats, json_scalar_to_string};
use crate::model::{Ability, Runner, Stat};

const RUNNER_STAT_META_FIELDS: &[&str] = &[
    "season_id",
    "season_name",
    "season_version",
    "patch_version",
    "patch_notes",
    "season_type",
    "release_date",
    "is_current",
];

#[must_use]
pub fn marathondb_runner_to_model(slug: &str, data: &Value) -> Runner {
    let name = data.get("name").and_then(Value::as_str).map(str::to_string);
    let role = data.get("role").and_then(Value::as_str).map(str::to_string);
    let description =
        data.get("description").and_then(Value::as_str).map(str::to_string);
    let portrait_url =
        data.get("portrait_url").and_then(Value::as_str).map(str::to_string);

    let abilities = data
        .get("abilities")
        .and_then(Value::as_array)
        .map(|abilities| {
            abilities
                .iter()
                .filter_map(|a| {
                    let name = a.get("name").and_then(Value::as_str)?.to_string();
                    Some(Ability {
                        ability_type: a
                            .get("ability_type")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        name,
                        description: a
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        cooldown_seconds: a
                            .get("cooldown_seconds")
                            .and_then(Value::as_u64)
                            .and_then(|n| u32::try_from(n).ok()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let stats = data
        .get("stats")
        .and_then(current_or_last_stats)
        .map(|s| {
            s.iter()
                .filter(|(k, _)| !RUNNER_STAT_META_FIELDS.contains(&k.as_str()))
                .filter_map(|(k, v)| {
                    json_scalar_to_string(v)
                        .map(|v| Stat { name: k.clone(), value: v })
                })
                .collect()
        })
        .unwrap_or_default();

    Runner {
        slug: slug.to_string(),
        name: name.unwrap_or_else(|| slug.to_string()),
        role,
        description,
        portrait_url,
        abilities,
        cores: Vec::new(), // not exposed by the MarathonDB runner endpoint
        stats,
    }
}
