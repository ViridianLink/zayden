pub mod commands;
pub mod error;
pub mod images;
pub mod kind;
pub mod settings;

pub use commands::{register, run};
pub use error::{GreetingsError, Result};
pub use images::{GreetingImage, MAX_URL_LEN, validate_url};
pub use kind::GreetingKind;
pub use serenity::all::GuildId;
pub use settings::{
    GreetingsConfig,
    GreetingsSettings,
    GreetingsStore,
    MAX_MESSAGE_LEN,
    render,
};
