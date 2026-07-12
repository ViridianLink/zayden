mod autocomplete;
mod command;

pub use autocomplete::Destiny2 as Destiny2Autocomplete;
pub use command::Destiny2;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Destiny2).add_autocomplete(Destiny2Autocomplete);
}
