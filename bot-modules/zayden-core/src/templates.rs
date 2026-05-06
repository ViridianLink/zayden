use std::fmt::{Display, Write};

use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};

const ITEMS_PER_PAGE: usize = 10;

pub fn leaderboard<'a, T: Display>(
    title: &'a str,
    data: impl Iterator<Item = T>,
    page: u32,
) -> CreateEmbed<'a> {
    let page_offset = (page.saturating_sub(1) as usize) * ITEMS_PER_PAGE;

    let description = data
        .enumerate()
        .fold(String::new(), |mut output, (i, item)| {
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

            write!(output, "{place_marker} - {item}").unwrap();
            output
        });

    CreateEmbed::new()
        .title(title)
        .description(description)
        .footer(CreateEmbedFooter::new(format!("Page {page}")))
        .colour(Colour::TEAL)
}
