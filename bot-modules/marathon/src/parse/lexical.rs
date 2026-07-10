use serde_json::Value;

use crate::model::Stat;

pub(super) fn node_plain_text(node: &Value) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

fn collect_text(node: &Value, out: &mut String) {
    let Some(obj) = node.as_object() else {
        return;
    };

    match obj.get("type").and_then(Value::as_str) {
        Some("text") => {
            if let Some(t) = obj.get("text").and_then(Value::as_str) {
                out.push_str(t);
            }
        },
        Some("static-data-widget") => {
            if let Some(label) = obj.get("label").and_then(Value::as_str) {
                out.push_str(label);
            }
        },
        _ => {},
    }

    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        for child in children {
            collect_text(child, out);
        }
    }
}

pub(super) fn tables_in_widget(widget_data: &Value) -> Vec<Vec<Vec<String>>> {
    let mut out = Vec::new();
    if let Some(root) = widget_data.pointer("/contentV2/root") {
        find_tables(root, &mut out);
    }
    out
}

fn find_tables(node: &Value, out: &mut Vec<Vec<Vec<String>>>) {
    let Some(obj) = node.as_object() else {
        return;
    };

    if obj.get("type").and_then(Value::as_str) == Some("table") {
        let mut rows = Vec::new();
        if let Some(row_nodes) = obj.get("children").and_then(Value::as_array) {
            for row in row_nodes {
                if row.get("type").and_then(Value::as_str) != Some("tablerow") {
                    continue;
                }
                let mut cells = Vec::new();
                if let Some(cell_nodes) =
                    row.get("children").and_then(Value::as_array)
                {
                    for cell in cell_nodes {
                        cells.push(node_plain_text(cell).trim().to_string());
                    }
                }
                rows.push(cells);
            }
        }
        out.push(rows);
        return;
    }

    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        for child in children {
            find_tables(child, out);
        }
    }
}

pub(super) fn stats_from_widget(widget_data: &Value) -> Vec<Stat> {
    tables_in_widget(widget_data)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let mut cells = row.into_iter();
            let name = cells.next()?;
            let value = cells.next()?;
            Some(Stat { name, value })
        })
        .collect()
}

pub(super) fn stat_value<'a>(stats: &'a [Stat], name: &str) -> Option<&'a str> {
    stats
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .map(|s| s.value.as_str())
        .filter(|v| !v.is_empty())
}

pub(super) fn single_cell_rows(widget_data: &Value) -> Vec<String> {
    tables_in_widget(widget_data)
        .into_iter()
        .flatten()
        .filter_map(|row| if row.len() == 1 { row.into_iter().next() } else { None })
        .collect()
}

pub(super) fn first_image_src(widget_data: &Value) -> Option<String> {
    let root = widget_data.pointer("/contentV2/root")?;
    find_image_src(root)
}

fn find_image_src(node: &Value) -> Option<String> {
    let obj = node.as_object()?;

    if obj.get("type").and_then(Value::as_str) == Some("image") {
        return obj.get("src").and_then(Value::as_str).map(str::to_string);
    }

    obj.get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(find_image_src)
}

pub(super) fn plain_paragraphs(widget_data: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(root) = widget_data.pointer("/contentV2/root") {
        collect_paragraphs(root, &mut out);
    }
    out
}

fn collect_paragraphs(node: &Value, out: &mut Vec<String>) {
    let Some(obj) = node.as_object() else {
        return;
    };

    match obj.get("type").and_then(Value::as_str) {
        Some("table") => return,
        Some("paragraph" | "listitem") => {
            let text = node_plain_text(node);
            let text = text.trim();
            if !text.is_empty() {
                out.push(text.to_string());
            }
            return;
        },
        _ => {},
    }

    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        for child in children {
            collect_paragraphs(child, out);
        }
    }
}

pub(super) fn static_widgets(widget_data: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(root) = widget_data.pointer("/contentV2/root") {
        collect_static_widgets(root, &mut out);
    }
    out
}

fn collect_static_widgets(node: &Value, out: &mut Vec<(String, String)>) {
    let Some(obj) = node.as_object() else {
        return;
    };

    if obj.get("type").and_then(Value::as_str) == Some("static-data-widget") {
        let label = obj.get("label").and_then(Value::as_str).unwrap_or_default();
        let group = obj.get("groupId").and_then(Value::as_str).unwrap_or_default();
        out.push((label.to_string(), group.to_string()));
    }

    if let Some(children) = obj.get("children").and_then(Value::as_array) {
        for child in children {
            collect_static_widgets(child, out);
        }
    }
}

pub(super) fn content_array(doc_data: &Value) -> &[Value] {
    doc_data.get("content").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

pub(super) fn widget_title(widget: &Value) -> Option<&str> {
    widget.pointer("/data/title").and_then(Value::as_str)
}

pub(super) fn widget_data(widget: &Value) -> &Value {
    widget.get("data").unwrap_or(&Value::Null)
}

pub(super) fn is_content_widget(widget: &Value) -> bool {
    matches!(
        widget.get("__typename").and_then(Value::as_str),
        Some(
            "NgfDocumentCmWidgetRichTextV2"
                | "NgfDocumentCmWidgetRichTextSimplifiedV2"
        )
    )
}

pub(super) fn find_widget<'a>(
    content: &'a [Value],
    title: &str,
) -> Option<&'a Value> {
    content
        .iter()
        .filter(|w| is_content_widget(w))
        .find(|w| widget_title(w).is_some_and(|t| t.eq_ignore_ascii_case(title)))
}

pub(super) fn find_widget_containing<'a>(
    content: &'a [Value],
    needle: &str,
) -> Option<&'a Value> {
    let needle = needle.to_ascii_lowercase();
    content.iter().filter(|w| is_content_widget(w)).find(|w| {
        widget_title(w).is_some_and(|t| t.to_ascii_lowercase().contains(&needle))
    })
}

pub(super) fn header_fields(
    content: &[Value],
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(header) = content.iter().find(|w| {
        w.get("__typename").and_then(Value::as_str)
            == Some("NgfDocumentStWidgetHeaderV2")
    }) else {
        return (None, None, None);
    };

    let data = widget_data(header);
    let name = data.get("title").and_then(Value::as_str).map(str::to_string);
    let description = data.pointer("/optionalSubTitleV2/root").and_then(|root| {
        let mut paras = Vec::new();
        collect_paragraphs(root, &mut paras);
        paras.into_iter().next()
    });
    let thumbnail =
        data.get("backgroundCss").and_then(Value::as_str).map(str::to_string);

    (name, description, thumbnail)
}

pub(super) fn slugify(s: &str) -> String {
    let cleaned: String = s
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    cleaned.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-")
}

pub(super) fn non_empty<S: AsRef<str>>(s: S) -> Option<String> {
    let trimmed = s.as_ref().trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn leading_number(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let end = trimmed
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(trimmed.len());
    trimmed.get(..end).and_then(non_empty)
}
