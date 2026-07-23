mod commands;

pub use commands::{Levels, Rank, Xp};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Levels)
        .add_command(Rank)
        .add_command(Xp)
        .add_component(Levels)?;

    Ok(())
}
