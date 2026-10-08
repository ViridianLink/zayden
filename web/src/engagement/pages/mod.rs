mod action;
mod fields;
pub mod greetings;
mod header;
mod levels;
mod reaction_roles;
mod state;

use topcoat::router::RouterBuilder;

pub const LEVELS_TITLE: &str = "Levels - Zayden Dashboard";
pub const REACTION_ROLES_TITLE: &str = "Reaction roles - Zayden Dashboard";
pub const GREETINGS_TITLE: &str = "Greetings - Zayden Dashboard";

/// The last segment of every form address on the reaction roles page: the
/// legacy `?action=` names.
#[must_use]
pub fn reaction_role_actions() -> Vec<&'static str> {
    reaction_roles::action_names()
}

/// The last segment of every form address on the greetings page: the legacy
/// `?action=` names, then the header's module switch.
#[must_use]
pub fn greeting_actions() -> Vec<&'static str> {
    greetings::action_names()
}

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(levels::levels)
        .page(reaction_roles::show)
        .page(reaction_roles::legacy_submit)
        .page(reaction_roles::submit)
        .page(greetings::show)
        .page(greetings::legacy_submit)
        .page(greetings::submit)
}
