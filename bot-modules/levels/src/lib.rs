pub mod commands;
pub mod common;
pub mod components;
pub mod error;
pub use commands::{Rank, Xp};
pub use components::LevelsCustomId;
pub use error::{LevelsError, Result};

pub mod message_create;
pub use message_create::message_create;

pub mod manager;
pub use manager::{FullLevelRow, LeaderboardRow, LevelsRow, RankRow, XpRow};

pub struct Levels;

#[must_use]
pub const fn level_up_xp(level: i32) -> i32 {
    (3 * level * level) + (50 * level) + 100
}
