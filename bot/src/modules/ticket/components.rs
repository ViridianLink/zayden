use serenity::all::{ComponentInteraction, CreateInputText, Http, InputTextStyle};
use ticket::TicketComponent;

use crate::Result;

use super::Ticket;

impl Ticket {
    pub async fn ticket_create(http: &Http, component: &ComponentInteraction) -> Result<()> {
        let version =
            CreateInputText::new(InputTextStyle::Short, "Version", "version").placeholder("1.0.0");

        let additional = CreateInputText::new(
            InputTextStyle::Paragraph,
            "Additional Information",
            "additional",
        )
        .placeholder("Please provide any additional information that may help us assist you.")
        .required(false);

        TicketComponent::ticket_create(http, component, [version, additional]).await?;

        Ok(())
    }
}
