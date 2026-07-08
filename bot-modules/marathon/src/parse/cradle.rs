use serde_json::Value;

use super::lexical::{
    content_array,
    find_widget_containing,
    plain_paragraphs,
    single_cell_rows,
    widget_data,
};
use crate::model::{Cradle, CradleNode};

#[must_use]
pub fn parse_cradle(doc_data: &Value) -> Cradle {
    let content = content_array(doc_data);

    let description = find_widget_containing(content, "cradle")
        .map(|w| plain_paragraphs(widget_data(w)).join(" "))
        .filter(|s| !s.is_empty());

    let nodes = find_widget_containing(content, "node")
        .map(|w| {
            single_cell_rows(widget_data(w))
                .into_iter()
                .map(|name| CradleNode { name, description: None })
                .collect()
        })
        .unwrap_or_default();

    Cradle { description, nodes }
}
