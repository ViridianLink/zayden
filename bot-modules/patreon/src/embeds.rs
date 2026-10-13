use std::fmt::Write as _;

use serenity::all::{
    Colour,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateTextDisplay,
};
use zayden_core::announce::{escape, render};
use zayden_core::templates::{body_component, separator, text};

use crate::content;
use crate::store::{PatreonAnnounceRow, PendingPost};

/// Patreon's brand coral.
const ACCENT: Colour = Colour::from_rgb(0xff, 0x42, 0x4d);

const UNTITLED: &str = "New Patreon post";

pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("{roles}", "the roles to ping"),
    ("{title}", "the post's title"),
    ("{url}", "the post's link"),
    ("{access}", "\"Public post\" or \"Patrons only\""),
];

const fn access(post: &PendingPost) -> &'static str {
    if post.is_public { "Public post" } else { "Patrons only" }
}

/// The guild's own text above the post card, empty when it has none.
#[must_use]
pub fn intro_content(post: &PendingPost, row: &PatreonAnnounceRow) -> String {
    let title = escape(post.title.as_deref().unwrap_or(UNTITLED));

    render(row.message.as_deref().unwrap_or_default(), &row.ping_role_ids, &[
        ("title", &title),
        ("url", &post.url),
        ("access", access(post)),
    ])
}

#[must_use]
pub fn intro_component(
    post: &PendingPost,
    row: &PatreonAnnounceRow,
) -> Option<CreateComponent<'static>> {
    let content = intro_content(post, row);

    (!content.is_empty())
        .then(|| CreateComponent::TextDisplay(CreateTextDisplay::new(content)))
}

pub fn post_component(post: &PendingPost) -> CreateComponent<'static> {
    let title = post.title.as_deref().unwrap_or(UNTITLED);
    let access = access(post);

    let mut header = format!(
        "## {title}\n-# Patreon \u{b7} {access} \u{b7} <t:{}:R>",
        post.published_at.to_jiff().as_second()
    );

    let body = post
        .content_html
        .as_deref()
        .filter(|_| post.is_public)
        .map(content::to_discord);

    let mut components: Vec<CreateContainerComponent<'static>> =
        Vec::with_capacity(4);

    match body.as_deref().filter(|body| !body.is_empty()) {
        Some(body) => {
            components.push(text(header));
            components.push(body_component(
                body.to_owned(),
                post.thumbnail_url.as_deref(),
            ));
        },
        None => {
            let note = if post.is_public {
                "_(no post text)_"
            } else {
                "_(patrons only \u{2014} become a patron to read it on Patreon)_"
            };
            let _ = write!(header, "\n\n-# {note}");
            components.push(body_component(header, post.thumbnail_url.as_deref()));
        },
    }

    components.push(separator());
    components.push(text(post.url.clone()));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}
