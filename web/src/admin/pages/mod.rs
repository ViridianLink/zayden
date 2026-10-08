mod error;
pub mod gate;
pub mod loadouts;
pub mod servers;

use topcoat::router::RouterBuilder;

pub const SERVERS_TITLE: &str = "All bot servers - Zayden Dashboard";
pub const LOADOUTS_TITLE: &str = "Loadout builder - Zayden Dashboard";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(servers::servers).page(loadouts::loadouts_page).page(loadouts::delete)
}
