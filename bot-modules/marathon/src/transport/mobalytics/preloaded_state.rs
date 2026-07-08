use serde_json::Value;

use crate::error::{MarathonError, Result};

const PRELOADED_STATE_MARKER: &str = "window.__PRELOADED_STATE__";

pub(super) fn contains_preloaded_state(html: &str) -> bool {
    html.contains(PRELOADED_STATE_MARKER)
}

pub(super) fn extract_marathon_state(html: &str) -> Result<Value> {
    let marker_start = html.find(PRELOADED_STATE_MARKER).ok_or_else(|| {
        MarathonError::Parse("__PRELOADED_STATE__ not found in page".to_string())
    })?;
    let after_marker = html
        .get(marker_start + PRELOADED_STATE_MARKER.len()..)
        .ok_or_else(|| {
            MarathonError::Parse("internal slicing error after marker".to_string())
        })?;

    let eq_pos = after_marker.find('=').ok_or_else(|| {
        MarathonError::Parse("malformed __PRELOADED_STATE__ assignment".to_string())
    })?;
    let after_eq = after_marker
        .get(eq_pos + 1..)
        .ok_or_else(|| {
            MarathonError::Parse("internal slicing error after `=`".to_string())
        })?
        .trim_start();

    let brace_start = after_eq.find('{').ok_or_else(|| {
        MarathonError::Parse("no JSON object after __PRELOADED_STATE__".to_string())
    })?;
    let json_slice = after_eq.get(brace_start..).ok_or_else(|| {
        MarathonError::Parse("internal slicing error at JSON start".to_string())
    })?;

    let json_text = extract_balanced_json(json_slice)?;
    let full: Value = serde_json::from_str(json_text)
        .map_err(|e| MarathonError::Parse(e.to_string()))?;
    full.get("marathonState").cloned().ok_or_else(|| {
        MarathonError::Parse("__PRELOADED_STATE__ has no marathonState".to_string())
    })
}

fn extract_balanced_json(s: &str) -> Result<&str> {
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
                        MarathonError::Parse(
                            "internal slicing error at JSON end".to_string(),
                        )
                    });
                }
            },
            _ => {},
        }
    }

    Err(MarathonError::Parse("unbalanced JSON in __PRELOADED_STATE__".to_string()))
}
