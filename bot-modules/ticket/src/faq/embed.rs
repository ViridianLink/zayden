use serenity::all::{CreateEmbed, CreateEmbedFooter};

use crate::faq::markdown::{self, DESCRIPTION_LIMIT};
use crate::wiki::{Page, SearchResult, WikiConfig};

const FOOTER: &str = "Source: wiki";
const NO_RESULTS: &str =
    "Nothing in the wiki matched that. Try a different app or service name.";
const RESULTS_TITLE: &str = "Possible matches";

pub fn answer(
    config: &WikiConfig,
    page: &Page,
    answer: &str,
) -> CreateEmbed<'static> {
    article(config, &page.title, &page.path)
        .description(markdown::truncate(answer, DESCRIPTION_LIMIT))
}

pub fn page(config: &WikiConfig, page: &Page) -> CreateEmbed<'static> {
    let content = markdown::for_discord(&page.content, config);

    let embed = article(config, &page.title, &page.path)
        .description(markdown::truncate(&content, DESCRIPTION_LIMIT));

    match markdown::take_first_image(&page.content, config) {
        Some(image) => embed.image(image.to_string(), None),
        None => embed,
    }
}

pub fn results(
    config: &WikiConfig,
    results: &[SearchResult],
) -> CreateEmbed<'static> {
    if results.is_empty() {
        return CreateEmbed::new()
            .title(RESULTS_TITLE)
            .description(NO_RESULTS)
            .footer(CreateEmbedFooter::new(FOOTER));
    }

    let body = results
        .iter()
        .map(|result| link_line(config, result))
        .collect::<Vec<_>>()
        .join("\n");

    CreateEmbed::new()
        .title(RESULTS_TITLE)
        .description(markdown::truncate(&body, DESCRIPTION_LIMIT))
        .footer(CreateEmbedFooter::new(FOOTER))
}

pub(super) fn link_line(config: &WikiConfig, result: &SearchResult) -> String {
    match config.article_url(&result.path) {
        Ok(url) => {
            format!("\u{1f539} [{}]({url})\n> {}", result.title, result.description)
        },
        Err(_e) => format!("\u{1f539} {}\n> {}", result.title, result.description),
    }
}

fn article(config: &WikiConfig, title: &str, path: &str) -> CreateEmbed<'static> {
    let embed = CreateEmbed::new()
        .title(title.to_owned())
        .footer(CreateEmbedFooter::new(FOOTER));

    match config.article_url(path) {
        Ok(url) => embed.url(url.to_string()),
        Err(_e) => embed,
    }
}
