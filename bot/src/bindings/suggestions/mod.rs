mod components;
pub mod slash_command;

pub use slash_command::FetchSuggestions;

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(FetchSuggestions)
        .add_component(components::SuggestionsAccept)?
        .add_component(components::SuggestionsReject)?
        .add_modal(components::SuggestionsAcceptModal)?
        .add_modal(components::SuggestionsRejectModal)?;

    Ok(())
}
