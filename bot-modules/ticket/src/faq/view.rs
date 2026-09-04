use serenity::all::{
    Colour,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
};

use crate::faq::article::FaqArticle;
use crate::faq::hit::{FaqHit, FaqSource};
use crate::faq::render::{self, BODY_LIMIT};
use crate::wiki::{Page, WikiConfig};

const ACCENT: Colour = Colour::new(0x00_99_ff);
const WIKI_FOOTER: &str = "-# Source: wiki";
const LOCAL_FOOTER: &str = "-# Source: server FAQ";
const OPEN_ARTICLE: &str = "Open article";
const RESULTS_TITLE: &str = "## Possible matches";
const NO_RESULTS: &str =
    "Nothing in the wiki matched that. Try a different app or service name.";

pub(crate) fn answer(
    config: &WikiConfig,
    page: &Page,
    answer: &str,
) -> CreateComponent<'static> {
    let body = render::truncate(answer, BODY_LIMIT);

    container(
        header(&page.title, Some(&page.path), None),
        &body,
        WIKI_FOOTER,
        link(config, &page.path),
    )
}

pub(crate) fn page(
    config: &WikiConfig,
    page: &Page,
    query: &str,
    anchor: Option<&str>,
) -> CreateComponent<'static> {
    let body = render::excerpt(&page.content, config, query, anchor, BODY_LIMIT);
    let thumbnail = render::thumbnail(&page.content, config);

    container(
        header(&page.title, Some(&page.path), thumbnail.map(Into::into)),
        &body,
        WIKI_FOOTER,
        link(config, &page.path),
    )
}

pub(crate) fn local_answer(
    stored: &FaqArticle,
    answer: &str,
) -> CreateComponent<'static> {
    local(stored, &render::truncate(answer, BODY_LIMIT))
}

pub(crate) fn stored(stored: &FaqArticle) -> CreateComponent<'static> {
    let sections = render::split_sections(&stored.content);
    let body = render::fit(&sections, BODY_LIMIT);

    local(stored, &body)
}

pub(crate) fn results(
    config: &WikiConfig,
    hits: &[FaqHit],
) -> CreateComponent<'static> {
    if hits.is_empty() {
        return CreateComponent::Container(
            CreateContainer::new(vec![text(RESULTS_TITLE), text(NO_RESULTS)])
                .accent_colour(ACCENT),
        );
    }

    let body = hits.iter().map(|hit| link_line(config, hit)).collect::<Vec<_>>();

    CreateComponent::Container(
        CreateContainer::new(vec![
            text(RESULTS_TITLE),
            divider(),
            text(render::truncate(&body.join("\n"), BODY_LIMIT)),
            divider(),
            text(WIKI_FOOTER),
        ])
        .accent_colour(ACCENT),
    )
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

fn local(stored: &FaqArticle, body: &str) -> CreateComponent<'static> {
    let footer = match stored.tags.as_slice() {
        [] => LOCAL_FOOTER.to_owned(),
        tags => format!("{LOCAL_FOOTER} | {}", tags.join(", ")),
    };

    container(header(&stored.title, None, None), body, &footer, None)
}

fn container(
    header: CreateContainerComponent<'static>,
    body: &str,
    footer: &str,
    link: Option<CreateContainerComponent<'static>>,
) -> CreateComponent<'static> {
    let mut components = vec![header, divider(), text(body.to_owned())];

    if let Some(link) = link {
        components.push(divider());
        components.push(link);
    }

    components.push(text(footer.to_owned()));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

fn header(
    title: &str,
    path: Option<&str>,
    thumbnail: Option<String>,
) -> CreateContainerComponent<'static> {
    let heading = path.map_or_else(
        || format!("## {title}"),
        |path| format!("## {title}\n-# {path}"),
    );

    let Some(thumbnail) = thumbnail else {
        return text(heading);
    };

    CreateContainerComponent::Section(CreateSection::new(
        vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(heading))],
        CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
            CreateUnfurledMediaItem::new(thumbnail),
        )),
    ))
}

fn link(
    config: &WikiConfig,
    path: &str,
) -> Option<CreateContainerComponent<'static>> {
    let url = config.article_url(path).ok()?;

    Some(CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new_link(url.to_string()).label(OPEN_ARTICLE),
    ])))
}

fn text(content: impl Into<String>) -> CreateContainerComponent<'static> {
    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content.into()))
}

fn divider() -> CreateContainerComponent<'static> {
    CreateContainerComponent::Separator(CreateSeparator::new().divider(true))
}
