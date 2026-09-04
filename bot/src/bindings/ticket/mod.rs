use crate::RegistryBuilder;
use crate::registry::OverlapError;

mod autocomplete;

pub mod components;
pub mod events;
pub mod message_commands;
pub mod slash_commands;

use autocomplete::TicketAutocomplete;
use components::{
    CreateTicketModal,
    SupportClose,
    SupportFaq,
    SupportTicket,
    TicketCreate,
};
use slash_commands::TicketCommand;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(TicketCommand)
        .add_autocomplete(TicketAutocomplete)
        .add_component(TicketCreate)?
        .add_component(SupportTicket)?
        .add_component(SupportClose)?
        .add_component(SupportFaq)?
        .add_modal(CreateTicketModal)?;

    Ok(())
}
