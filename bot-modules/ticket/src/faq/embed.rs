use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};

use crate::faq::article::FaqArticle;
use crate::faq::hit::{FaqHit, FaqSource};
use crate::faq::markdown::{self, DESCRIPTION_LIMIT};
use crate::wiki::{Page, WikiConfig};

const FOOTER: &str = "Source: wiki";
const LOCAL_FOOTER: &str = "Source: server FAQ";
const CREATED_FOOTER: &str =
    "Staff can edit or remove this in the dashboard under Support, FAQ.";
const NO_RESULTS: &str =
    "Nothing in the wiki matched that. Try a different app or service name.";
const RESULTS_TITLE: &str = "Possible matches";
pub(crate) const CREATED_TITLE: &str = "FAQ article created";
const CREATED_COLOUR: Colour = Colour::new(0x00_99_ff);

pub(crate) fn answer(
    config: &WikiConfig,
    page: &Page,
    answer: &str,
) -> CreateEmbed<'static> {
    article(config, &page.title, &page.path)
        .description(markdown::truncate(answer, DESCRIPTION_LIMIT))
}

pub(crate) fn page(config: &WikiConfig, page: &Page) -> CreateEmbed<'static> {
    let content = markdown::for_discord(&page.content, config);

    let embed = article(config, &page.title, &page.path)
        .description(markdown::truncate(&content, DESCRIPTION_LIMIT));

    match markdown::take_first_image(&page.content, config) {
        Some(image) => embed.image(image.to_string(), None),
        None => embed,
    }
}

pub(crate) fn local_answer(
    stored: &FaqArticle,
    answer: &str,
) -> CreateEmbed<'static> {
    local(stored).description(markdown::truncate(answer, DESCRIPTION_LIMIT))
}

pub(crate) fn stored(stored: &FaqArticle) -> CreateEmbed<'static> {
    local(stored).description(markdown::truncate(&stored.content, DESCRIPTION_LIMIT))
}

pub(crate) fn created(stored: &FaqArticle) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title(CREATED_TITLE)
        .colour(CREATED_COLOUR)
        .description(format!("**{}**\n{}", stored.title, stored.summary))
        .footer(CreateEmbedFooter::new(CREATED_FOOTER))
}

pub(crate) fn results(config: &WikiConfig, hits: &[FaqHit]) -> CreateEmbed<'static> {
    if hits.is_empty() {
        return CreateEmbed::new()
            .title(RESULTS_TITLE)
            .description(NO_RESULTS)
            .footer(CreateEmbedFooter::new(FOOTER));
    }

    let body =
        hits.iter().map(|hit| link_line(config, hit)).collect::<Vec<_>>().join("\n");

    CreateEmbed::new()
        .title(RESULTS_TITLE)
        .description(markdown::truncate(&body, DESCRIPTION_LIMIT))
        .footer(CreateEmbedFooter::new(FOOTER))
}

pub(crate) fn link_line(config: &WikiConfig, hit: &FaqHit) -> String {
    let heading = match hit.source {
        FaqSource::Local { .. } => format!("**{}**", hit.title),
        FaqSource::Wiki => match config.article_url(&hit.path) {
            Ok(url) => format!("[{}]({url})", hit.title),
            Err(_e) => hit.title.clone(),
        },
    };

    format!("\u{1f539} {heading}\n> {}", hit.description)
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

fn local(stored: &FaqArticle) -> CreateEmbed<'static> {
    let footer = match stored.tags.as_slice() {
        [] => LOCAL_FOOTER.to_owned(),
        tags => format!("{LOCAL_FOOTER} | {}", tags.join(", ")),
    };

    CreateEmbed::new()
        .title(stored.title.clone())
        .footer(CreateEmbedFooter::new(footer))
}
