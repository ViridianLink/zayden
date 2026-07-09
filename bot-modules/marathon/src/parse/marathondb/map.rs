use std::collections::HashMap;

use serde_json::Value;

use crate::model::{Location, LootRoom, MapStatus, MarathonMap, Poi};

#[must_use]
pub fn marathondb_map_to_model(slug: &str, data: &Value) -> MarathonMap {
    let name = data
        .get("name")
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    let map_image_url =
        data.get("image_url").and_then(Value::as_str).map(str::to_string);

    let status = data
        .get("is_active")
        .and_then(Value::as_bool)
        .map(|active| if active { MapStatus::Available } else { MapStatus::Locked });

    let zones = zone_names(data);

    let pois = data
        .get("zones")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|zone| {
            Some(Poi {
                name: zone.get("name").and_then(Value::as_str)?.to_string(),
                description: string_field(zone, "description"),
            })
        })
        .collect();

    let pois_array =
        data.get("pois").and_then(Value::as_array).cloned().unwrap_or_default();

    let extractions = locations_in_category(&pois_array, "extraction", &zones);
    let event_spawns = locations_in_category(&pois_array, "activity", &zones);

    let keycard_rooms = dedup(
        pois_array
            .iter()
            .filter(|poi| {
                poi.get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|t| t.ends_with("_key"))
            })
            .filter_map(|poi| {
                Some(LootRoom {
                    name: poi_label(poi)?,
                    location_hint: poi_hint(poi, &zones),
                })
            }),
        |room| (room.name.clone(), room.location_hint.clone()),
    );

    MarathonMap {
        slug: slug.to_string(),
        name,
        status,
        map_image_url,
        pois,
        extractions,
        event_spawns,
        keycard_rooms,
    }
}

fn locations_in_category(
    pois: &[Value],
    category: &str,
    zones: &HashMap<i64, String>,
) -> Vec<Location> {
    dedup(
        pois.iter()
            .filter(|poi| {
                poi.get("category").and_then(Value::as_str) == Some(category)
            })
            .filter_map(|poi| {
                Some(Location {
                    name: poi_label(poi)?,
                    description: poi_hint(poi, zones),
                })
            }),
        |loc| (loc.name.clone(), loc.description.clone()),
    )
}

fn poi_label(poi: &Value) -> Option<String> {
    string_field(poi, "name").or_else(|| string_field(poi, "type_label"))
}

fn poi_hint(poi: &Value, zones: &HashMap<i64, String>) -> Option<String> {
    string_field(poi, "description").or_else(|| {
        poi.get("zone_id")
            .and_then(Value::as_i64)
            .and_then(|id| zones.get(&id).cloned())
    })
}

fn zone_names(data: &Value) -> HashMap<i64, String> {
    data.get("zones")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|zone| {
            let id = zone.get("id").and_then(Value::as_i64)?;
            let name = string_field(zone, "name")?;
            Some((id, name))
        })
        .collect()
}

fn dedup<T, K, F>(items: impl Iterator<Item = T>, key: F) -> Vec<T>
where
    K: PartialEq,
    F: Fn(&T) -> K,
{
    let mut seen: Vec<K> = Vec::new();
    let mut out: Vec<T> = Vec::new();
    for item in items {
        let k = key(&item);
        if !seen.contains(&k) {
            seen.push(k);
            out.push(item);
        }
    }
    out
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
