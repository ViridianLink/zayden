pub mod attachment;
pub mod commands;
pub mod cooldown;
pub mod error;
pub mod images;
pub mod kind;
pub mod settings;

pub use commands::{register, run};
pub use cooldown::{COOLDOWNS, Verdict, verdict};
pub use error::{GreetingsError, Result};
pub use images::{GreetingImage, MAX_URL_LEN, validate_url};
pub use kind::GreetingKind;
pub use serenity::all::GuildId;
pub use settings::{
    GreetingsConfig,
    GreetingsSettings,
    GreetingsStore,
    MAX_MESSAGE_LEN,
    parse_cooldown,
    render,
};
pub use zayden_app::config::{Cooldowns, GreetingsSettingsRow};
