pub use slash_command::ReactionRoleCommand;

pub mod slash_command;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(ReactionRoleCommand);
}
