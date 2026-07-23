mod components;
pub mod error;
pub mod manager;
mod modal;
mod reaction;
pub mod slash_command;

use error::Result;
pub use error::SuggestionsError;
pub use manager::SuggestionsGuildRow;
pub use reaction::{ReviewAction, review_action};
pub use slash_command::FetchSuggestions;

pub struct Suggestions;
