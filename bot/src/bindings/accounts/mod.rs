mod command;
mod components;

pub use command::{Link, Unlink};
use components::{UnlinkCancel, UnlinkConfirm};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Link)
        .add_command(Unlink)
        .add_component(UnlinkConfirm)?
        .add_component(UnlinkCancel)?;

    Ok(())
}
