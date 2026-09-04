mod code;
mod html;
mod links;
mod sections;
mod spans;
mod tables;
mod truncate;

pub use links::thumbnail;
pub use sections::{Section, anchor, best_match, fit, split_sections};
pub use truncate::truncate;

use crate::faq::render::spans::{Span, blocks, map_text};
use crate::wiki::WikiConfig;

pub const BODY_LIMIT: usize = 3600;
pub const PROMPT_LIMIT: usize = 16_000;

#[must_use]
pub fn for_discord(content: &str, config: &WikiConfig) -> String {
    blocks(content)
        .into_iter()
        .map(|span| match span {
            Span::Code(block) => code::normalize(block),
            Span::Text(text) => prose(text, config),
        })
        .collect::<Vec<_>>()
        .concat()
}

fn prose(text: &str, config: &WikiConfig) -> String {
    let inline = map_text(text, |chunk| {
        let chunk = html::strip(chunk);
        let chunk = links::strip_images(&chunk);
        let chunk = links::strip_attribute_blocks(&chunk);

        links::absolutize(&chunk, config)
    });

    code::fence_indented(&tables::reflow(&inline))
}

#[must_use]
pub fn excerpt(
    content: &str,
    config: &WikiConfig,
    query: &str,
    anchor: Option<&str>,
    limit: usize,
) -> String {
    let rendered = for_discord(content, config);
    let sections = split_sections(&rendered);

    if sections.is_empty() {
        return truncate(&rendered, limit);
    }

    let chosen = anchor.map_or_else(
        || best_match(&sections, query),
        |anchor| sections.iter().find(|section| section.anchor == anchor),
    );

    chosen.map_or_else(
        || fit(&sections, limit),
        |section| fit(std::slice::from_ref(section), limit),
    )
}
