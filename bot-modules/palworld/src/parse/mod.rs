mod palcalc;
mod paldex;

use std::collections::HashMap;

pub use palcalc::pal_from_palcalc;
pub use paldex::{item_from_raw, pal_from_raw, passive_from_raw};
use scraper::{Html, Selector};

use crate::model::Element;

fn meta_content(html: &str, property: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let selector =
        Selector::parse(&format!(r#"meta[property="{property}"]"#)).ok()?;
    let content =
        doc.select(&selector).next()?.value().attr("content")?.trim().to_string();
    (!content.is_empty()).then_some(content)
}

#[must_use]
pub fn og_description(html: &str) -> Option<String> {
    meta_content(html, "og:description")
}

#[must_use]
pub fn og_image(html: &str) -> Option<String> {
    meta_content(html, "og:image")
}

#[must_use]
pub fn pal_elements_index(html: &str) -> HashMap<String, Vec<Element>> {
    let doc = Html::parse_document(html);
    let (Ok(card), Ok(link), Ok(icon)) = (
        Selector::parse("div.pal"),
        Selector::parse(r#"a[href^="/pal/"]"#),
        Selector::parse(".elements .element img"),
    ) else {
        return HashMap::new();
    };

    let mut map = HashMap::new();
    for pal in doc.select(&card) {
        let Some(slug) = pal
            .select(&link)
            .next()
            .and_then(|a| a.value().attr("href"))
            .and_then(|href| href.strip_prefix("/pal/"))
            .map(str::to_string)
        else {
            continue;
        };

        let elements: Vec<Element> = pal
            .select(&icon)
            .filter_map(|img| img.value().attr("alt"))
            .filter_map(|alt| Element::parse(alt.trim_end_matches(" element")))
            .collect();

        if !elements.is_empty() {
            map.entry(slug).or_insert(elements);
        }
    }
    map
}

#[must_use]
pub fn gg_slug(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' => Some(c),
            ' ' | '-' | '_' => Some('-'),
            _ => None,
        })
        .collect()
}
