use scraper::{ElementRef, Html, Selector};
use serde_json::Value;

use crate::error::{MarathonError, Result};

#[must_use]
pub fn document(html: &str) -> Html {
    Html::parse_document(html)
}

fn selector(css: &str) -> Result<Selector> {
    Selector::parse(css)
        .map_err(|e| MarathonError::Parse(format!("invalid selector `{css}`: {e}")))
}

/// Concatenate an element's descendant text and collapse runs of whitespace.
fn element_text(el: ElementRef<'_>) -> String {
    let joined = el.text().collect::<Vec<_>>().join(" ");
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn text_of(doc: &Html, css: &str) -> Result<Option<String>> {
    let sel = selector(css)?;
    Ok(doc.select(&sel).next().and_then(|el| {
        let cleaned = element_text(el);
        (!cleaned.is_empty()).then_some(cleaned)
    }))
}

/// For every element matching `row_css`, pair the text of its first two
/// descendant `<span>` elements as `(label, value)`.
///
/// This suits the common "stat row" shape used by server-rendered dashboards
/// (`<div><span>Label</span><span>Value</span>…</div>`); a trailing third span
/// such as a tooltip is ignored. Rows without two spans, or whose label or
/// value is blank, are skipped.
pub fn span_pairs(doc: &Html, row_css: &str) -> Result<Vec<(String, String)>> {
    let rows = selector(row_css)?;
    let span = selector("span")?;
    let mut out = Vec::new();
    for row in doc.select(&rows) {
        let mut spans = row.select(&span);
        let (Some(label), Some(value)) = (spans.next(), spans.next()) else {
            continue;
        };
        let label = element_text(label);
        let value = element_text(value);
        if !label.is_empty() && !value.is_empty() {
            out.push((label, value));
        }
    }
    Ok(out)
}

pub fn attr_of(doc: &Html, css: &str, attr: &str) -> Result<Option<String>> {
    let sel = selector(css)?;
    Ok(doc
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr(attr))
        .map(str::to_string))
}

pub fn script_json_by_selector(doc: &Html, css: &str) -> Result<Value> {
    let sel = selector(css)?;
    let body: String = doc
        .select(&sel)
        .next()
        .ok_or_else(|| MarathonError::Parse(format!("no element matched `{css}`")))?
        .text()
        .collect();
    serde_json::from_str(&body).map_err(|e| MarathonError::Parse(e.to_string()))
}

pub fn script_json_after_marker(doc: &Html, marker: &str) -> Result<Value> {
    let script = selector("script")?;
    for el in doc.select(&script) {
        let body: String = el.text().collect();
        if let Some((_, rest)) = body.split_once(marker) {
            let json = balanced_object(rest)?;
            return serde_json::from_str(json)
                .map_err(|e| MarathonError::Parse(e.to_string()));
        }
    }
    Err(MarathonError::Parse(format!("no <script> contained `{marker}`")))
}

#[must_use]
pub fn next_flight(doc: &Html) -> String {
    let Ok(script) = selector("script") else { return String::new() };
    let mut flight = String::new();
    for el in doc.select(&script) {
        let body: String = el.text().collect();
        for rest in body.split(".push([1,").skip(1) {
            if let Some(chunk) = leading_json_string(rest) {
                flight.push_str(&chunk);
            }
        }
    }
    flight
}

fn leading_json_string(s: &str) -> Option<String> {
    let start = s.find('"')?;
    let bytes = s.as_bytes();
    let mut i = start + 1;
    let mut escaped = false;
    while let Some(&b) = bytes.get(i) {
        if escaped {
            escaped = false;
        } else if b == b'\\' {
            escaped = true;
        } else if b == b'"' {
            let literal = s.get(start..=i)?;
            return serde_json::from_str::<String>(literal).ok();
        }
        i += 1;
    }
    None
}

#[must_use]
pub fn flight_object_by_slug(flight: &str, slug: &str) -> Option<Value> {
    const MARKER: &str = "{\"id\":\"";
    let needle = format!("\"slug\":\"{slug}\"");

    let mut best: Option<Value> = None;
    let mut best_keys = 0usize;
    let mut best_refs = usize::MAX;
    let mut from = 0usize;

    while let Some(rel) = flight.get(from..).and_then(|tail| tail.find(MARKER)) {
        let start = from + rel;
        from = start + MARKER.len();

        let Some(tail) = flight.get(start..) else { break };
        let Ok(obj) = balanced_object(tail) else { continue };
        if !obj.contains(&needle) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(obj) else { continue };
        if value.get("slug").and_then(Value::as_str) != Some(slug) {
            continue;
        }

        let keys = value.as_object().map_or(0, serde_json::Map::len);
        let refs = count_chunk_refs(&value);
        if keys > best_keys || (keys == best_keys && refs < best_refs) {
            best_keys = keys;
            best_refs = refs;
            best = Some(value);
        }
    }
    best
}

fn count_chunk_refs(value: &Value) -> usize {
    value.as_object().map_or(0, |obj| {
        obj.values().filter(|v| v.as_str().is_some_and(is_chunk_ref)).count()
    })
}

fn is_chunk_ref(s: &str) -> bool {
    s.strip_prefix('$').is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

fn balanced_object(s: &str) -> Result<&str> {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, b) in s.bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return s.get(..=i).ok_or_else(|| {
                        MarathonError::Parse("json object slicing error".into())
                    });
                }
            },
            _ => {},
        }
    }

    Err(MarathonError::Parse("unbalanced json object".into()))
}
