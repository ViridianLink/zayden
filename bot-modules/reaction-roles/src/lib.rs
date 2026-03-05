mod command;
pub mod error;
mod reaction;
pub mod reaction_roles_manager;

pub use command::ReactionRoleCommand;
pub use error::{Error, Result};
pub use reaction::ReactionRoleReaction;
pub use reaction_roles_manager::ReactionRolesManager;
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
