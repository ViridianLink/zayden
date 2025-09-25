pub mod commands;
pub mod common;
pub mod components;
pub use commands::{Rank, Xp};

pub mod message_create;
pub use message_create::message_create;

pub mod sqlx_lib;
pub use sqlx_lib::{FullLevelRow, LeaderboardRow, LevelsManager, LevelsRow, RankRow, XpRow};

pub struct Levels;

#[inline(always)]
pub const fn level_up_xp(level: i32) -> i32 {
    (3 * level * level) + (50 * level) + 100
}
