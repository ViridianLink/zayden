use serde_json::Value;

use super::lexical::{
    content_array,
    first_image_src,
    header_fields,
    is_content_widget,
    tables_in_widget,
    widget_data,
    widget_title,
};
use crate::model::{Location, LootRoom, MarathonMap, Poi};

#[must_use]
pub fn parse_map(slug: &str, doc_data: &Value) -> MarathonMap {
    let content = content_array(doc_data);
    let (name, _description, _thumbnail) = header_fields(content);

    let mut map_image_url = None;
    let mut pois = Vec::new();
    let mut extractions = Vec::new();
    let mut event_spawns = Vec::new();
    let mut keycard_rooms = Vec::new();

    for widget in content {
        if !is_content_widget(widget) {
            continue;
        }
        let Some(title) = widget_title(widget) else {
            continue;
        };
        let lower = title.to_ascii_lowercase();
        let is_relevant =
            ["spawn", "exfil", "supply", "hazard", "intercept", "danger"]
                .iter()
                .any(|kw| lower.contains(kw));
        if !is_relevant {
            continue;
        }

        if map_image_url.is_none()
            && lower.contains("spawn")
            && lower.contains("exfil")
        {
            map_image_url = first_image_src(widget_data(widget));
        }

        for row in tables_in_widget(widget_data(widget)).into_iter().flatten() {
            let mut cells = row.into_iter();
            let Some(entry_name) = cells.next() else { continue };
            let Some(count) = cells.next() else { continue };

            let description = if count.is_empty() {
                None
            } else {
                Some(format!("{count} on this map"))
            };
            let entry_lower = entry_name.to_ascii_lowercase();

            if entry_lower.contains("exfil") {
                extractions.push(Location { name: entry_name, description });
            } else if entry_lower.contains("hazard")
                || entry_lower.contains("supply")
            {
                keycard_rooms
                    .push(LootRoom { name: entry_name, location_hint: description });
            } else if entry_lower.contains("intercept") {
                event_spawns.push(Location { name: entry_name, description });
            } else {
                pois.push(Poi { name: entry_name, description });
            }
        }
    }

    MarathonMap {
        slug: slug.to_string(),
        name: name.unwrap_or_else(|| slug.to_string()),
        status: None,
        map_image_url,
        pois,
        extractions,
        event_spawns,
        keycard_rooms,
    }
}
