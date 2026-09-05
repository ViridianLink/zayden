use std::fmt::{Display, Write};

use serenity::all::{
    Colour,
    CreateContainerComponent,
    CreateEmbed,
    CreateEmbedFooter,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
};

const ITEMS_PER_PAGE: usize = 10;

pub fn leaderboard<T: Display>(
    title: &str,
    data: impl Iterator<Item = T>,
    page: u32,
) -> CreateEmbed<'_> {
    let page_offset = (page.saturating_sub(1) as usize) * ITEMS_PER_PAGE;

    let description =
        data.enumerate().fold(String::new(), |mut output, (i, item)| {
            if !output.is_empty() {
                output.push_str("\n\n");
            }

            let place = page_offset + i + 1;

            let place_marker = match place {
                1 => "🥇".to_string(),
                2 => "🥈".to_string(),
                3 => "🥉".to_string(),
                _ => format!("#{place}"),
            };

            let _ = write!(output, "{place_marker} - {item}");
            output
        });

    CreateEmbed::new()
        .title(title)
        .description(description)
        .footer(CreateEmbedFooter::new(format!("Page {page}")))
        .colour(Colour::TEAL)
}

pub fn separator() -> CreateContainerComponent<'static> {
    CreateContainerComponent::Separator(CreateSeparator::new().divider(true))
}

pub fn text(content: impl Into<String>) -> CreateContainerComponent<'static> {
    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content.into()))
}

pub fn body_component(
    content: String,
    thumbnail_url: Option<&str>,
) -> CreateContainerComponent<'static> {
    match thumbnail_url {
        Some(url) => CreateContainerComponent::Section(CreateSection::new(
            vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                content,
            ))],
            CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
                CreateUnfurledMediaItem::new(url.to_string()),
            )),
        )),
        None => text(content),
    }
}
