use std::collections::HashMap;

use serde_json::Value;

use crate::merge::dedup;
use crate::model::{Location, LootRoom, MarathonMap, Poi};

#[must_use]
pub fn mapgenie_map_to_model(
    slug: &str,
    taxonomy: &Value,
    data: &Value,
) -> MarathonMap {
    let name = data
        .get("map")
        .and_then(|m| m.get("title"))
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    let categories = category_titles(taxonomy);

    let mut pois = Vec::new();
    let mut extractions = Vec::new();
    let mut event_spawns = Vec::new();
    let mut keycard_rooms = Vec::new();

    for loc in data.get("locations").and_then(Value::as_array).into_iter().flatten()
    {
        let Some(name) = loc.get("title").and_then(Value::as_str).map(title_case)
        else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let description = string_field(loc, "description");
        let category = loc
            .get("category_id")
            .and_then(Value::as_i64)
            .and_then(|id| categories.get(&id))
            .map_or("", String::as_str);

        match bucket(category) {
            Bucket::Extraction => extractions.push(Location { name, description }),
            Bucket::Keycard => {
                keycard_rooms.push(LootRoom { name, location_hint: description });
            },
            Bucket::Event => event_spawns.push(Location { name, description }),
            Bucket::Poi => pois.push(Poi { name, description }),
        }
    }

    MarathonMap {
        slug: slug.to_string(),
        name,
        status: None,
        map_image_url: None,
        pois: dedup(pois, |p| p.name.to_lowercase()),
        extractions: dedup(extractions, |l| l.name.to_lowercase()),
        event_spawns: dedup(event_spawns, |l| l.name.to_lowercase()),
        keycard_rooms: dedup(keycard_rooms, |r| r.name.to_lowercase()),
    }
}

enum Bucket {
    Extraction,
    Keycard,
    Event,
    Poi,
}

fn bucket(category: &str) -> Bucket {
    let category = category.to_lowercase();
    if category.contains("exfil") {
        Bucket::Extraction
    } else if category.contains("access card") || category.contains("locked room") {
        Bucket::Keycard
    } else if category.contains("spawn")
        || category.contains("activity")
        || category.contains("contract")
    {
        Bucket::Event
    } else {
        Bucket::Poi
    }
}

fn category_titles(taxonomy: &Value) -> HashMap<i64, String> {
    let categories = match taxonomy.get("categories") {
        Some(Value::Object(map)) => map.values().collect::<Vec<_>>(),
        Some(Value::Array(array)) => array.iter().collect(),
        _ => Vec::new(),
    };

    categories
        .into_iter()
        .filter_map(|c| {
            let id = c.get("id").and_then(Value::as_i64)?;
            let title = c.get("title").and_then(Value::as_str)?.to_string();
            Some((id, title))
        })
        .collect()
}

fn title_case(raw: &str) -> String {
    raw.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.as_str().to_lowercase().chars())
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
