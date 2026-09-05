use std::fmt::Write as _;

use serenity::all::{
    Colour,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
};
use zayden_core::templates::{body_component, separator, text};

use crate::content;
use crate::store::PendingPost;

/// Patreon's brand coral.
const ACCENT: Colour = Colour::from_rgb(0xff, 0x42, 0x4d);

const UNTITLED: &str = "New Patreon post";

pub fn post_component(post: &PendingPost) -> CreateComponent<'static> {
    let title = post.title.as_deref().unwrap_or(UNTITLED);
    let access = if post.is_public { "Public post" } else { "Patrons only" };

    let mut header = format!(
        "## {title}\n-# Patreon \u{b7} {access} \u{b7} <t:{}:R>",
        post.published_at.to_jiff().as_second()
    );

    let body = post.content_html.as_deref().map(content::to_discord);

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
        // With no body there is nothing to sit beside the thumbnail, so the
        // heading itself carries the accessory.
        None => {
            let _ = write!(header, "\n\n-# _(no post text)_");
            components.push(body_component(header, post.thumbnail_url.as_deref()));
        },
    }

    components.push(separator());
    components.push(text(post.url.clone()));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}
