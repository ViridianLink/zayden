mod autocomplete;
mod command;

pub use autocomplete::Marathon as MarathonAutocomplete;
pub use command::Marathon;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Marathon).add_autocomplete(MarathonAutocomplete);
}
