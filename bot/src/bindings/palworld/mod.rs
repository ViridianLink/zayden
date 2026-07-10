mod autocomplete;
mod command;

pub use autocomplete::Palworld as PalworldAutocomplete;
pub use command::Palworld;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Palworld).add_autocomplete(PalworldAutocomplete);
}
