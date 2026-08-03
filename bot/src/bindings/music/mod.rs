mod command;
mod components;

pub use command::Music;
pub use components::{ControlPanel, QueuePager};

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder.add_command(Music);
    builder.add_component(ControlPanel)?;
    builder.add_component(QueuePager)?;

    Ok(())
}
