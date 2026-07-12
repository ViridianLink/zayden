use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub mod components;
pub mod message_commands;
pub mod slash_commands;

use components::{
    CreateTicketModal,
    SupportClose,
    SupportFaq,
    SupportTicket,
    TicketCreate,
};
use slash_commands::{SupportCommand, TicketCommand};

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(TicketCommand)
        .add_command(SupportCommand)
        .add_component(TicketCreate)?
        .add_component(SupportTicket)?
        .add_component(SupportClose)?
        .add_component(SupportFaq)?
        .add_modal(CreateTicketModal)?;

    Ok(())
}
