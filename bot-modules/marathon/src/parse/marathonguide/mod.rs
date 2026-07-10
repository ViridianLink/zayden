mod runner;
mod weapon;

pub use runner::marathonguide_html_to_runner;
use scraper::{ElementRef, Selector};
pub use weapon::marathonguide_html_to_weapon;

use crate::parse::html;
use crate::parse::lexical::non_empty;

pub(super) const BASE: &str = "https://marathon-guide.com";

pub(super) fn select_text(el: ElementRef<'_>, css: &str) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    el.select(&sel).next().map(html::element_text).and_then(non_empty)
}

pub(super) fn select_attr(
    el: ElementRef<'_>,
    css: &str,
    attr: &str,
) -> Option<String> {
    let sel = Selector::parse(css).ok()?;
    el.select(&sel)
        .next()
        .and_then(|node| node.value().attr(attr))
        .and_then(non_empty)
}

pub(super) fn absolute(path: &str) -> String {
    if path.starts_with("http") { path.to_string() } else { format!("{BASE}{path}") }
}
