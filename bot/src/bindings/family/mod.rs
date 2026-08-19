mod command;
mod components;

pub use command::Family;
use components::{AdoptAccept, AdoptDecline, MarryAccept, MarryDecline};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Family)
        .add_component(MarryAccept)?
        .add_component(MarryDecline)?
        .add_component(AdoptAccept)?
        .add_component(AdoptDecline)?;

    Ok(())
}
