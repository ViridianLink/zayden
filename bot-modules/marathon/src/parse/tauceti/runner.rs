use serde_json::Value;

use super::{field, humanize, scalar};
use crate::model::{Ability, Runner, Stat};

#[must_use]
pub fn tauceti_runner_to_model(slug: &str, value: &Value) -> Runner {
    let name = field(value, "name").unwrap_or_else(|| slug.to_string());

    Runner {
        slug: slug.to_string(),
        name,
        role: field(value, "playstyle"),
        description: field(value, "description"),
        portrait_url: field(value, "imageUrl")
            .or_else(|| field(value, "fullBodyUrl")),
        abilities: abilities(value),
        cores: Vec::new(),
        stats: base_stats(value),
    }
}

fn abilities(value: &Value) -> Vec<Ability> {
    let mut out = Vec::new();
    let frames = value.get("runners").and_then(Value::as_array);
    for frame in frames.into_iter().flatten() {
        let list = frame.get("abilities").and_then(Value::as_array);
        for a in list.into_iter().flatten() {
            let Some(name) = field(a, "name") else { continue };
            out.push(Ability {
                ability_type: field(a, "abilityType"),
                name,
                description: field(a, "description"),
                cooldown_seconds: a
                    .get("cooldown")
                    .and_then(Value::as_u64)
                    .and_then(|c| u32::try_from(c).ok()),
            });
        }
    }
    out
}

fn base_stats(value: &Value) -> Vec<Stat> {
    value
        .get("baseStats")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, raw)| {
            scalar(raw).map(|value| Stat { name: humanize(key), value })
        })
        .collect()
}
