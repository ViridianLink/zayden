mod faction;
mod runner;
mod weapon;

pub use faction::tauceti_faction_to_model;
pub use runner::tauceti_runner_to_model;
use serde_json::Value;
pub use weapon::tauceti_item_to_weapon;

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_string())
        },
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(scalar)
}

fn humanize(raw: &str) -> String {
    let mut spaced = String::with_capacity(raw.len() + 4);
    let mut prev_joinable = false;
    for ch in raw.chars() {
        match ch {
            '_' | '-' | ' ' => {
                spaced.push(' ');
                prev_joinable = false;
            },
            c if c.is_ascii_uppercase() && prev_joinable => {
                spaced.push(' ');
                spaced.push(c);
                prev_joinable = false;
            },
            c => {
                spaced.push(c);
                prev_joinable = c.is_ascii_lowercase() || c.is_ascii_digit();
            },
        }
    }

    spaced
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
