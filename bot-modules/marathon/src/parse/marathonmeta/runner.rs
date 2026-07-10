use super::{BASE, identity};
use crate::model::{Runner, Stat};
use crate::parse::html;

const STAT_ROWS: &str = "div[class*=\"gap-1\"]";
const DESCRIPTION: &str = "p[class*=\"max-w-3xl\"]";

#[must_use]
pub fn marathonmeta_html_to_runner(slug: &str, rendered: &str) -> Runner {
    let doc = html::document(rendered);
    let ident = identity(&doc, slug);

    let description =
        html::text_of(&doc, DESCRIPTION).ok().flatten().or(ident.description);

    Runner {
        slug: slug.to_string(),
        name: ident.name,
        role: ident.category,
        description,
        portrait_url: Some(format!("{BASE}/assets/runners/{slug}.png")),
        abilities: Vec::new(),
        cores: Vec::new(),
        stats: stats(&doc),
    }
}

fn stats(doc: &scraper::Html) -> Vec<Stat> {
    html::span_pairs(doc, STAT_ROWS)
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, value)| is_numeric(value))
        .map(|(name, value)| Stat { name, value })
        .collect()
}

fn is_numeric(value: &str) -> bool {
    let body = value.strip_prefix('-').unwrap_or(value);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}
