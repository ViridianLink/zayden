use serde_json::Value;

use super::lexical::{
    content_array,
    find_widget_containing,
    header_fields,
    single_cell_rows,
    slugify,
    tables_in_widget,
    widget_data,
};
use crate::model::{Contract, Faction, Upgrade};

#[must_use]
pub fn parse_faction_listing(doc_data: &Value) -> Vec<Faction> {
    let content = content_array(doc_data);
    let Some(widget) = content.iter().find(|w| {
        w.get("__typename").and_then(Value::as_str)
            == Some("NgfDocumentCmWidgetCardGridV2")
    }) else {
        return Vec::new();
    };

    widget_data(widget)
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let name = item.get("title").and_then(Value::as_str)?.to_string();
            let slug = item
                .get("linkUrl")
                .and_then(Value::as_str)
                .and_then(|url| url.rsplit('/').next())
                .map_or_else(|| slugify(&name), str::to_string);
            Some(Faction {
                slug,
                name,
                priority_contracts: Vec::new(),
                upgrades: Vec::new(),
            })
        })
        .collect()
}

#[must_use]
pub fn parse_faction(slug: &str, doc_data: &Value) -> Faction {
    let content = content_array(doc_data);
    let (name, _description, _thumbnail) = header_fields(content);

    let priority_contracts = find_widget_containing(content, "contract")
        .map(|w| {
            single_cell_rows(widget_data(w))
                .into_iter()
                .filter(|line| !line.eq_ignore_ascii_case("contract"))
                .map(|line| Contract {
                    slug: slugify(&line),
                    name: line,
                    description: None,
                    difficulty: None,
                })
                .collect()
        })
        .unwrap_or_default();

    let upgrades = find_widget_containing(content, "upgrade")
        .map(|w| {
            tables_in_widget(widget_data(w))
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    let mut cells = row.into_iter();
                    let name = cells.next()?;
                    if name.eq_ignore_ascii_case("upgrade") {
                        return None;
                    }
                    let cost = cells.next();
                    let requirements = cells.next();
                    Some(Upgrade { name, cost, requirements })
                })
                .collect()
        })
        .unwrap_or_default();

    Faction {
        slug: slug.to_string(),
        name: name.unwrap_or_else(|| slug.to_string()),
        priority_contracts,
        upgrades,
    }
}
