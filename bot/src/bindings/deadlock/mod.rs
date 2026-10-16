mod autocomplete;
mod command;

pub use autocomplete::Deadlock as DeadlockAutocomplete;
pub use command::Deadlock;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Deadlock).add_autocomplete(DeadlockAutocomplete);
}
