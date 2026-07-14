mod autocomplete;
mod command;
mod modal;

pub use autocomplete::Palworld as PalworldAutocomplete;
pub use command::Palworld;
use modal::PalworldUploadModal;

use crate::RegistryBuilder;
use crate::registry::OverlapError;

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Palworld)
        .add_autocomplete(PalworldAutocomplete)
        .add_modal(PalworldUploadModal)?;

    Ok(())
}
