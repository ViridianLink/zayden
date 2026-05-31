pub mod commands;
pub mod error;
pub mod manager;

pub use commands::{GiveStar, Stars};
pub use error::GoldStarError;
use error::Result;
pub use manager::{GoldStarManager, GoldStarRow};
