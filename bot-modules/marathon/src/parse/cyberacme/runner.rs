use serde_json::Value;

use super::scalar;
use crate::model::Runner;

#[must_use]
pub fn cyberacme_runner_to_model(slug: &str, value: &Value) -> Runner {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map_or_else(|| slug.to_string(), str::to_string);

    // "Unknown" taglines are placeholders, not a real role.
    let role = value
        .get("tagline")
        .and_then(scalar)
        .filter(|t| !t.eq_ignore_ascii_case("unknown"));

    Runner {
        slug: slug.to_string(),
        name,
        role,
        description: None,
        portrait_url: value.get("imagePath").and_then(scalar),
        abilities: Vec::new(),
        cores: Vec::new(),
        stats: Vec::new(),
    }
}
