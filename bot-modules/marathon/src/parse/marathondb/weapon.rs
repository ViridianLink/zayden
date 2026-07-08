use serde_json::Value;

use super::{current_or_last_stats, json_scalar_to_string};
use crate::model::{Stat, Weapon};

const WEAPON_STAT_META_FIELDS: &[&str] = &[
    "season_id",
    "season_name",
    "season_version",
    "patch_version",
    "patch_notes",
    "weapon_patch_notes",
    "season_type",
    "release_date",
    "is_current",
];

#[must_use]
pub fn marathondb_weapon_to_model(slug: &str, data: &Value) -> Weapon {
    let name = data.get("name").and_then(Value::as_str).map(str::to_string);
    let weapon_type = data.get("type").and_then(Value::as_str).map(str::to_string);
    let ammo_type =
        data.get("ammo_type").and_then(Value::as_str).map(str::to_string);
    let description =
        data.get("description").and_then(Value::as_str).map(str::to_string);
    let thumbnail_url =
        data.get("icon_url").and_then(Value::as_str).map(str::to_string);

    let mut stats = Vec::new();
    let mut damage = None;
    let mut fire_rate = None;
    let mut magazine_size = None;
    let mut reload_speed = None;
    let mut range = None;

    if let Some(current) = data.get("stats").and_then(current_or_last_stats) {
        for (key, value) in current {
            if WEAPON_STAT_META_FIELDS.contains(&key.as_str()) {
                continue;
            }
            let Some(v) = json_scalar_to_string(value) else { continue };
            match key.as_str() {
                "damage" => damage = Some(v.clone()),
                "rate_of_fire" => fire_rate = Some(v.clone()),
                "magazine_size" => magazine_size = Some(v.clone()),
                "reload_speed" => reload_speed = Some(v.clone()),
                "range_meters" => range = Some(v.clone()),
                _ => {},
            }
            stats.push(Stat { name: key.clone(), value: v });
        }
    }

    Weapon {
        slug: slug.to_string(),
        name: name.unwrap_or_else(|| slug.to_string()),
        weapon_type,
        ammo_type,
        damage,
        fire_rate,
        magazine_size,
        reload_speed,
        range,
        description,
        thumbnail_url,
        stats,
        attachment_slots: Vec::new(), /* MarathonDB doesn't expose mod/attachment
                                       * data */
    }
}
