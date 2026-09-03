use std::collections::HashMap;

use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    CreateButton,
    CreateEmbed,
    CreateMessage,
    EditInteractionResponse,
    Http,
    ResolvedValue,
};
use zayden_core::required_option;

use crate::{Result, Ticket};

impl Ticket {
    pub(super) async fn create(
        http: &Http,
        interaction: &CommandInteraction,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        let title: &str = required_option(&mut options, "title")?;
        let description: &str = required_option(&mut options, "description")?;
        let label: &str = required_option(&mut options, "label")?;

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
