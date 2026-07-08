use serde_json::Value;

use super::lexical::{
    content_array,
    find_widget,
    find_widget_containing,
    header_fields,
    single_cell_rows,
    stats_from_widget,
    tables_in_widget,
    widget_data,
};
use crate::model::{Ability, Runner};

#[must_use]
pub fn parse_runner(slug: &str, doc_data: &Value) -> Runner {
    let content = content_array(doc_data);
    let (name, description, portrait_url) = header_fields(content);

    let stats = find_widget(content, "Stats")
        .map(|w| stats_from_widget(widget_data(w)))
        .unwrap_or_default();

    let abilities = find_widget_containing(content, "abilities")
        .map(|w| parse_abilities(widget_data(w)))
        .unwrap_or_default();

    let cores = find_widget_containing(content, "cores")
        .map(|w| {
            tables_in_widget(widget_data(w))
                .into_iter()
                .flatten()
                .filter_map(|row| row.into_iter().next())
                .filter(|name| !name.eq_ignore_ascii_case("core"))
                .collect()
        })
        .unwrap_or_default();

    Runner {
        slug: slug.to_string(),
        name: name.unwrap_or_else(|| slug.to_string()),
        role: None,
        description,
        portrait_url,
        abilities,
        cores,
        stats,
    }
}

fn is_ability_type_marker(s: &str) -> bool {
    s == "Prime Ability" || s == "Tactical Ability" || s.starts_with("Trait ")
}

fn parse_leading_number(s: &str) -> Option<u32> {
    s.split_whitespace().next()?.parse().ok()
}

fn parse_abilities(widget_data: &Value) -> Vec<Ability> {
    let rows = single_cell_rows(widget_data);
    let mut abilities = Vec::new();
    let mut i = 0;

    while i < rows.len() {
        let Some(name) = rows.get(i) else { break };
        let Some(marker) = rows.get(i + 1) else { break };

        if !is_ability_type_marker(marker) {
            i += 1;
            continue;
        }

        let name = name.clone();
        let ability_type = Some(marker.clone());
        let mut j = i + 2;
        let mut cooldown_seconds = None;

        while let Some(row) = rows.get(j) {
            if let Some(rest) = row.strip_prefix("Base Cooldown:") {
                cooldown_seconds = parse_leading_number(rest);
            } else if !row.starts_with("Duration:") {
                break;
            }
            j += 1;
        }

        let description = rows.get(j).cloned();
        if description.is_some() {
            j += 1;
        }

        abilities.push(Ability {
            ability_type,
            name,
            description,
            cooldown_seconds,
        });
        i = j;
    }

    abilities
}
