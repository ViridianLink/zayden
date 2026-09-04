use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};

use crate::faq::article::FaqArticle;

const CREATED_FOOTER: &str =
    "Staff can edit or remove this in the dashboard under Support, FAQ.";
pub(crate) const CREATED_TITLE: &str = "FAQ article created";
const CREATED_COLOUR: Colour = Colour::new(0x00_99_ff);

pub(crate) fn created(stored: &FaqArticle) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title(CREATED_TITLE)
        .colour(CREATED_COLOUR)
        .description(format!("**{}**\n{}", stored.title, stored.summary))
        .footer(CreateEmbedFooter::new(CREATED_FOOTER))
}
