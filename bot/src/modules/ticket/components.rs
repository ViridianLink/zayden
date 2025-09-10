use serenity::all::{ComponentInteraction, Http};
use ticket::TicketComponent;

use crate::Result;

use super::Ticket;

impl Ticket {
    pub async fn ticket_create(http: &Http, component: &ComponentInteraction) -> Result<()> {
        TicketComponent::ticket_create(http, component, []).await?;

        Ok(())
    }
}
