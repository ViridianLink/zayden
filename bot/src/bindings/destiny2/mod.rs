mod autocomplete;
mod cache_listener;
mod command;

pub use autocomplete::Destiny2 as Destiny2Autocomplete;
pub use cache_listener::spawn_cache_listener;
pub use command::Destiny2;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Destiny2).add_autocomplete(Destiny2Autocomplete);
}
