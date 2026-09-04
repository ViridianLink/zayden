use std::collections::HashMap;

use serenity::all::{
    ButtonStyle,
    ChannelType,
    CommandInteraction,
    CreateButton,
    CreateEmbed,
    CreateMessage,
    EditInteractionResponse,
    GenericInteractionChannel,
    Http,
    ResolvedValue,
};
use zayden_core::required_option;

use crate::{Result, Ticket, TicketError};

impl Ticket {
    pub(super) async fn create(
        http: &Http,
        interaction: &CommandInteraction,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let title: &str = required_option(&mut options, "title")?;
        let description: &str = required_option(&mut options, "description")?;
        let label: &str = required_option(&mut options, "label")?;

        // Forum channels hold posts, not messages, so the send below would fail
        // with an opaque Discord error rather than something actionable.
        if let GenericInteractionChannel::Channel(channel) = &interaction.channel
            && channel.base.kind == ChannelType::Forum
        {
            return Err(TicketError::ForumChannelUnsupported);
        }

        interaction.defer_ephemeral(http).await?;

        let embed = CreateEmbed::new()
            .title(title)
            .description(description.replace("\\n", "\n"));

        let button = CreateButton::new("ticket_create")
            .style(ButtonStyle::Primary)
            .label(label);

        interaction
            .channel_id
            .send_message(http, CreateMessage::new().embed(embed).button(button))
            .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Ticket embed created"),
            )
            .await?;

        Ok(())
    }
}
