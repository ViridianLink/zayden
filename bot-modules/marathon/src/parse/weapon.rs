use serde_json::Value;

use super::lexical::{
    content_array,
    find_widget,
    find_widget_containing,
    header_fields,
    plain_paragraphs,
    slugify,
    stat_value,
    static_widgets,
    stats_from_widget,
    widget_data,
};
use crate::model::{Attachment, AttachmentSlot, Weapon};

#[must_use]
pub fn parse_weapon(slug: &str, doc_data: &Value) -> Weapon {
    let content = content_array(doc_data);
    let (name, description, thumbnail_url) = header_fields(content);

    let mut stats = Vec::new();
    if let Some(w) = find_widget(content, "Overview") {
        stats.extend(stats_from_widget(widget_data(w)));
    }
    if let Some(w) = find_widget(content, "Stats") {
        stats.extend(stats_from_widget(widget_data(w)));
    }

    let weapon_type = stat_value(&stats, "Type").map(str::to_string);
    let ammo_type = stat_value(&stats, "Ammo Type").map(str::to_string);
    let damage = stat_value(&stats, "Damage").map(str::to_string);
    let fire_rate = stat_value(&stats, "Rate of Fire").map(str::to_string);
    let magazine_size = stat_value(&stats, "Magazine Size").map(str::to_string);
    let reload_speed = stat_value(&stats, "Reload Speed").map(str::to_string);
    let range = stat_value(&stats, "Range").map(str::to_string);

    let attachment_slots = find_widget_containing(content, "compatible mods")
        .or_else(|| find_widget_containing(content, "attachment"))
        .map(|w| {
            parse_attachment_slots(widget_data(w), name.as_deref().unwrap_or(slug))
        })
        .unwrap_or_default();

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
        attachment_slots,
    }
}

fn parse_attachment_slots(
    widget_data: &Value,
    weapon_name: &str,
) -> Vec<AttachmentSlot> {
    let paras = plain_paragraphs(widget_data);
    let is_placeholder = paras.iter().any(|p| {
        p.eq_ignore_ascii_case("coming soon...")
            || p.eq_ignore_ascii_case("coming soon")
    });
    if is_placeholder {
        return Vec::new();
    }

    static_widgets(widget_data)
        .into_iter()
        .filter(|(label, _)| !label.is_empty())
        .map(|(label, group)| AttachmentSlot {
            slot: if group.is_empty() { "Mod".to_string() } else { group },
            attachment: Some(Attachment {
                slug: slugify(&label),
                name: label,
                slot: None,
                effect: None,
                compatible_weapons: vec![weapon_name.to_string()],
            }),
        })
        .collect()
}
