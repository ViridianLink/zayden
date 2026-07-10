mod runner;
mod weapon;

pub use runner::marathonmeta_html_to_runner;
pub use weapon::marathonmeta_html_to_weapon;

pub(super) const BASE: &str = "https://marathonmeta.gg";

pub(super) struct Identity {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}

pub(super) fn identity(doc: &scraper::Html, slug: &str) -> Identity {
    let raw = crate::parse::html::attr_of(
        doc,
        "meta[property=\"og:description\"]",
        "content",
    )
    .ok()
    .flatten();

    let Some(raw) = raw else {
        return Identity {
            name: slug.to_string(),
            category: None,
            description: None,
        };
    };

    let (head, description) = match raw.split_once(" — ") {
        Some((head, tail)) => (head.trim(), non_empty(tail)),
        None => (raw.trim(), None),
    };

    let (name, category) = match head.rsplit_once('(') {
        Some((name, rest)) => {
            let category = rest.strip_suffix(')').map(str::trim).and_then(non_empty);
            (name.trim(), category)
        },
        None => (head, None),
    };

    Identity {
        name: non_empty(name).unwrap_or_else(|| slug.to_string()),
        category,
        description,
    }
}

pub(super) fn leading_number(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let end = trimmed
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(trimmed.len());
    trimmed.get(..end).and_then(non_empty)
}

fn non_empty<S: AsRef<str>>(s: S) -> Option<String> {
    let trimmed = s.as_ref().trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
