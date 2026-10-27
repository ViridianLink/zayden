use topcoat::router::segment;
mod action;
mod fields;
pub mod greetings;
mod header;
mod levels;
mod reaction_roles;
mod state;

segment!(kind = Group);

pub const LEVELS_TITLE: &str = "Levels - Zayden Dashboard";
pub const REACTION_ROLES_TITLE: &str = "Reaction roles - Zayden Dashboard";
pub const GREETINGS_TITLE: &str = "Greetings - Zayden Dashboard";

#[must_use]
pub fn reaction_role_actions() -> Vec<&'static str> {
    reaction_roles::action_names()
}

#[must_use]
pub fn greeting_actions() -> Vec<&'static str> {
    greetings::action_names()
}
