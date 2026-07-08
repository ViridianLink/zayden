use serde_json::Value;

use super::lexical::{content_array, tables_in_widget, widget_data};
use crate::model::MetaEntry;

#[must_use]
pub fn parse_meta(doc_data: &Value) -> Vec<MetaEntry> {
    let content = content_array(doc_data);
    let mut entries = Vec::new();

    for widget in content {
        if widget.get("__typename").and_then(Value::as_str)
            == Some("NgfDocumentCmWidgetCardGridV2")
        {
            let items = widget_data(widget)
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten();
            for item in items {
                let Some(name) = item.get("title").and_then(Value::as_str) else {
                    continue;
                };
                entries.push(MetaEntry {
                    item: name.to_string(),
                    tier: item
                        .get("subtitle")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    note: None,
                });
            }
        }

        for row in tables_in_widget(widget_data(widget)).into_iter().flatten() {
            let mut cells = row.into_iter();
            let Some(item) = cells.next() else { continue };
            let tier = cells.next();
            let note = cells.next();
            entries.push(MetaEntry { item, tier, note });
        }
    }

    entries
}
