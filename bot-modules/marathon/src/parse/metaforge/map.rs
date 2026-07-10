use serde_json::Value;

use crate::merge::dedup;
use crate::model::{Location, LootRoom, MarathonMap, Poi};

#[must_use]
pub fn metaforge_markers_to_map(slug: &str, rows: &[Value]) -> MarathonMap {
    let mut pois = Vec::new();
    let mut extractions = Vec::new();
    let mut event_spawns = Vec::new();
    let mut keycard_rooms = Vec::new();

    for row in rows {
        let category =
            row.get("category").and_then(Value::as_str).unwrap_or_default();
        let subcategory =
            row.get("subcategory").and_then(Value::as_str).unwrap_or_default();

        let Some(bucket) = bucket(category, subcategory) else {
            continue;
        };
        let name = marker_name(row, subcategory);
        if name.is_empty() {
            continue;
        }

        match bucket {
            Bucket::Extraction => {
                extractions.push(Location { name, description: None });
            },
            Bucket::Keycard => {
                keycard_rooms.push(LootRoom { name, location_hint: None });
            },
            Bucket::Event => event_spawns.push(Location { name, description: None }),
            Bucket::Poi => pois.push(Poi { name, description: None }),
        }
    }

    MarathonMap {
        slug: slug.to_string(),
        name: humanize(slug),
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

fn bucket(category: &str, subcategory: &str) -> Option<Bucket> {
    match category {
        "locations" => Some(if subcategory.contains("exfil") {
            Bucket::Extraction
        } else if subcategory == "locked-room" || subcategory == "vault" {
            Bucket::Keycard
        } else {
            Bucket::Poi
        }),
        "activities" => Some(Bucket::Event),
        _ => None,
    }
}

fn marker_name(row: &Value, subcategory: &str) -> String {
    row.get("instance_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| humanize(subcategory), str::to_string)
}

fn humanize(raw: &str) -> String {
    raw.split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars.as_str().chars()).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
