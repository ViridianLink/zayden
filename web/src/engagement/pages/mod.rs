mod action;
pub mod greetings;
mod levels;
mod reaction_roles;

use topcoat::router::RouterBuilder;

pub const LEVELS_TITLE: &str = "Levels - Zayden Dashboard";
pub const REACTION_ROLES_TITLE: &str = "Reaction Roles - Zayden Dashboard";
pub const GREETINGS_TITLE: &str = "Greetings - Zayden Dashboard";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(levels::levels)
        .page(reaction_roles::show)
        .page(reaction_roles::submit)
        .page(greetings::show)
        .page(greetings::submit)
}
