mod error;
pub mod loadouts;
pub mod servers;

use topcoat::router::RouterBuilder;

pub const SERVERS_TITLE: &str = "All Servers - Zayden Dashboard";
pub const LOADOUTS_TITLE: &str = "Loadout Builder - Zayden Dashboard";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(servers::servers).page(loadouts::loadouts_page).page(loadouts::delete)
}
