use serde_json::Value;

use super::{field, humanize, scalar};
use crate::model::{Attachment, AttachmentSlot, Stat, Weapon};

const STAT_LABELS: &[(&str, &str)] = &[
    ("damage", "Damage"),
    ("firepower", "Firepower"),
    ("accuracy", "Accuracy"),
    ("handling", "Handling"),
    ("range", "Range"),
    ("magazine", "Magazine"),
    ("rateOfFire", "Rate of Fire"),
    ("reloadSpeed", "Reload Speed"),
    ("recoil", "Recoil"),
    ("zoom", "Zoom"),
    ("firingMode", "Firing Mode"),
];

#[must_use]
pub fn tauceti_item_to_weapon(slug: &str, item: &Value) -> Weapon {
    let name = field(item, "name").unwrap_or_else(|| slug.to_string());
    let stats_obj = item.get("stats");
    let stat = |key: &str| stats_obj.and_then(|s| s.get(key)).and_then(scalar);

    let stats: Vec<Stat> = STAT_LABELS
        .iter()
        .filter_map(|(key, label)| {
            stat(key).map(|value| Stat { name: (*label).to_string(), value })
        })
        .collect();

    Weapon {
        slug: slug.to_string(),
        name: name.clone(),
        weapon_type: field(item, "subcategory"),
        ammo_type: ammo_name(item),
        damage: stat("damage"),
        fire_rate: stat("rateOfFire"),
        magazine_size: stat("magazine"),
        reload_speed: stat("reloadSpeed"),
        range: stat("range"),
        description: field(item, "description"),
        thumbnail_url: field(item, "imageUrl"),
        stats,
        attachment_slots: mod_slots(item, &name),
    }
}

fn ammo_name(item: &Value) -> Option<String> {
    item.get("ammo")
        .and_then(|a| field(a, "name"))
        .or_else(|| item.get("ammoSlug").and_then(Value::as_str).map(humanize))
}

fn mod_slots(item: &Value, weapon_name: &str) -> Vec<AttachmentSlot> {
    item.get("activeModSlots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|raw| AttachmentSlot {
            slot: humanize(raw),
            attachment: Some(Attachment {
                slug: raw.to_string(),
                name: humanize(raw),
                slot: Some(humanize(raw)),
                effect: None,
                compatible_weapons: vec![weapon_name.to_string()],
            }),
        })
        .collect()
}
