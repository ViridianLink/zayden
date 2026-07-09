use serde_json::Value;

use super::{scalar, titleize};
use crate::model::{Attachment, AttachmentSlot, Stat, Weapon};

const STAT_LABELS: &[(&str, &str)] = &[
    ("firepower", "Firepower"),
    ("accuracy", "Accuracy"),
    ("handling", "Handling"),
    ("range", "Range"),
    ("magazineSize", "Magazine Size"),
    ("firingMode", "Firing Mode"),
    ("zoom", "Zoom"),
];

#[must_use]
pub fn cyberacme_item_to_weapon(slug: &str, item: &Value) -> Weapon {
    let name = item
        .get("name")
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    let variant =
        item.get("variants").and_then(Value::as_array).and_then(|v| v.first());
    let stats_obj = variant.and_then(|v| v.get("stats")).and_then(Value::as_object);

    let stat = |key: &str| stats_obj.and_then(|s| s.get(key)).and_then(scalar);

    let weapon_type = stat("weaponType")
        .or_else(|| item.get("weaponType").and_then(Value::as_str).map(titleize));

    let stats: Vec<Stat> = STAT_LABELS
        .iter()
        .filter_map(|(key, label)| {
            stat(key).map(|value| Stat { name: (*label).to_string(), value })
        })
        .collect();

    let description = variant
        .and_then(|v| v.get("description"))
        .and_then(scalar)
        .or_else(|| item.get("description").and_then(scalar));

    Weapon {
        slug: slug.to_string(),
        name: name.clone(),
        weapon_type,
        ammo_type: None,
        damage: stat("firepower"),
        fire_rate: None,
        magazine_size: stat("magazineSize"),
        reload_speed: None,
        range: stat("range"),
        description,
        thumbnail_url: item.get("imagePath").and_then(scalar),
        stats,
        attachment_slots: mod_slots(item, &name),
    }
}

fn mod_slots(item: &Value, weapon_name: &str) -> Vec<AttachmentSlot> {
    item.get("supportedModSlots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| AttachmentSlot {
            slot: titleize(raw),
            attachment: Some(Attachment {
                slug: raw.to_string(),
                name: titleize(raw),
                slot: Some(titleize(raw)),
                effect: None,
                compatible_weapons: vec![weapon_name.to_string()],
            }),
        })
        .collect()
}
