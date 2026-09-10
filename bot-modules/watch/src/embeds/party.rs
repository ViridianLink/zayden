use jellyfin::transport::JellyfinClient;
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateEmbed,
    CreateEmbedFooter,
};

use crate::components;
use crate::embeds::COLOUR;
use crate::party::row::PartyRow;

pub fn embed(
    client: &JellyfinClient,
    party: &PartyRow,
    guests: usize,
) -> CreateEmbed<'static> {
    let starts = party.starts_at().as_second();

    CreateEmbed::new()
        .title(format!("Watch party: {}", party.item_name))
        .colour(COLOUR)
        .description(format!(
            "Starts <t:{starts}:F> (<t:{starts}:R>)\n\
             [Open on Jellyfin]({})",
            client.item_url(&party.item_id)
        ))
        .field("Host", format!("<@{}>", party.host_id.cast_unsigned()), true)
        .field("Joined", guests.to_string(), true)
        .field("Party id", party.id.to_string(), true)
        .footer(CreateEmbedFooter::new(
            "Without a Jellyfin account, press Join and I will DM you temporary \
             access 30 minutes before the start.",
        ))
}

pub fn buttons(party_id: i64) -> CreateActionRow<'static> {
    CreateActionRow::buttons(vec![
        CreateButton::new(format!("{}{party_id}", components::PARTY_JOIN_PREFIX))
            .label("Join")
            .style(ButtonStyle::Primary),
        CreateButton::new(format!("{}{party_id}", components::PARTY_LEAVE_PREFIX))
            .label("Leave")
            .style(ButtonStyle::Secondary),
        CreateButton::new(format!("{}{party_id}", components::PARTY_CANCEL_PREFIX))
            .label("Cancel")
            .style(ButtonStyle::Danger),
    ])
}
