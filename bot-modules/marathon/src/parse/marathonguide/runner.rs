use scraper::{ElementRef, Selector};

use super::{absolute, select_text};
use crate::model::{Ability, Runner, Stat};
use crate::parse::html;
use crate::parse::lexical::non_empty;

#[must_use]
pub fn marathonguide_html_to_runner(slug: &str, page: &str) -> Runner {
    let doc = html::document(page);

    let name = html::text_of(&doc, "title")
        .ok()
        .flatten()
        .and_then(|title| non_empty(title.split(" - ").next().unwrap_or_default()))
        .unwrap_or_else(|| slug.to_string());

    Runner {
        slug: slug.to_string(),
        name,
        role: summary_tag(&doc, "Role"),
        description: None,
        portrait_url: select_attr_doc(&doc, "img.selected", "src")
            .as_deref()
            .map(absolute),
        abilities: abilities(&doc),
        cores: Vec::new(),
        stats: summary_stats(&doc),
    }
}

fn summary_tag(doc: &scraper::Html, label: &str) -> Option<String> {
    let sel = Selector::parse(&format!("span[mattooltip=\"{label}\"]")).ok()?;
    doc.select(&sel).next().map(html::element_text).and_then(non_empty)
}

fn summary_stats(doc: &scraper::Html) -> Vec<Stat> {
    ["Origin", "Tech"]
        .into_iter()
        .filter_map(|label| {
            summary_tag(doc, label)
                .map(|value| Stat { name: label.to_string(), value })
        })
        .collect()
}

fn abilities(doc: &scraper::Html) -> Vec<Ability> {
    let Ok(block) = Selector::parse("app-runner-ability") else {
        return Vec::new();
    };
    doc.select(&block).filter_map(ability).collect()
}

fn ability(block: ElementRef<'_>) -> Option<Ability> {
    let name = select_text(block, "div.text-primary")?;
    Some(Ability {
        ability_type: select_text(block, "div.font-semibold"),
        name,
        description: select_text(block, "div.opacity-75"),
        cooldown_seconds: None,
    })
}

fn select_attr_doc(doc: &scraper::Html, css: &str, attr: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|node| node.value().attr(attr))
        .and_then(non_empty)
}
