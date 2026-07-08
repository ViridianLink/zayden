use std::collections::BTreeSet;

use regex::Regex;

pub(super) fn extract_listing_slugs(html: &str, prefix: &str) -> Vec<String> {
    let pattern = format!(r#"href="/marathon/{prefix}/([a-z0-9-]+)""#);
    let Ok(re) = Regex::new(&pattern) else {
        return Vec::new();
    };

    let mut seen = BTreeSet::new();
    for cap in re.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            seen.insert(m.as_str().to_string());
        }
    }
    seen.into_iter().collect()
}
