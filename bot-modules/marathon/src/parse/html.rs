use scraper::{Html, Selector};
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

pub fn text_of(doc: &Html, css: &str) -> Result<Option<String>> {
    let sel = selector(css)?;
    Ok(doc.select(&sel).next().and_then(|el| {
        let joined = el.text().collect::<Vec<_>>().join(" ");
        let cleaned = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        (!cleaned.is_empty()).then_some(cleaned)
    }))
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
