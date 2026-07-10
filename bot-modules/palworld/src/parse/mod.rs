mod paldex;

pub use paldex::{item_from_raw, pal_from_raw, passive_from_raw};
use scraper::{Html, Selector};

#[must_use]
pub fn og_description(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse(r#"meta[property="og:description"]"#).ok()?;
    let content =
        doc.select(&selector).next()?.value().attr("content")?.trim().to_string();
    (!content.is_empty()).then_some(content)
}
