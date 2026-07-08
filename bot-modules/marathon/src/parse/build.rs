use serde_json::Value;

use super::lexical::{
    content_array,
    find_widget_containing,
    plain_paragraphs,
    widget_data,
};
use crate::model::BuildRecipe;

#[must_use]
pub fn parse_build(slug: &str, doc_data: &Value) -> BuildRecipe {
    let name = doc_data
        .pointer("/data/name")
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    let shell = doc_data
        .pointer("/tags/data")
        .and_then(Value::as_array)
        .and_then(|tags| {
            tags.iter().find(|t| {
                t.get("groupSlug").and_then(Value::as_str) == Some("runner")
            })
        })
        .and_then(|t| t.get("name").and_then(Value::as_str))
        .map(str::to_string);

    let gear = doc_data
        .pointer("/data/buildVariants/values")
        .and_then(Value::as_array)
        .and_then(|variants| {
            variants.iter().find_map(|variant| {
                variant.pointer("/genericBuilder/slots").and_then(Value::as_array)
            })
        })
        .map(|slots| {
            slots
                .iter()
                .filter_map(|slot| {
                    let slot_name =
                        slot.get("gameSlotSlug").and_then(Value::as_str)?;
                    let title =
                        slot.pointer("/gameEntity/title").and_then(Value::as_str)?;
                    Some(format!("{slot_name}: {title}"))
                })
                .collect()
        })
        .unwrap_or_default();

    let content = content_array(doc_data);
    let notes = find_widget_containing(content, "overview")
        .or_else(|| find_widget_containing(content, "how it works"))
        .map(|w| plain_paragraphs(widget_data(w)).join(" "))
        .filter(|s| !s.is_empty());

    BuildRecipe {
        slug: slug.to_string(),
        name,
        shell,
        cradle_focus: None,
        gear,
        notes,
    }
}
