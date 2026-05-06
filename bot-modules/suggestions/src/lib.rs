mod components;
pub mod error;
pub mod guild_manager;
mod modal;
mod reaction;
pub mod slash_command;

pub use error::Error;
use error::Result;
pub use guild_manager::{SuggestionsGuildManager, SuggestionsGuildRow};
pub use slash_command::FetchSuggestions;

pub struct Suggestions;
