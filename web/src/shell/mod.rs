//! The signed-in app frame and the pages that live only in it.
//!
//! Two frames wrap every members page. Both render
//! `div.app > nav.app-navbar + div.app-body > (aside.app-sidebar + main.app-main)`,
//! and a page renders only what goes inside `main`.
//!
//! - The guild layout wraps every page under `/guild/{guild_id}`. It adds the guild
//!   sidebar (server switcher, operator badge, module list) and answers a signed-out
//!   visitor with a 303 to `/login`. Pages under it read the guild with
//!   `path_param::<GuildId>(cx)` ([`GuildId`]) and must spell the parameter
//!   `{guild_id}`, or the layout does not wrap them.
//! - [`app_shell`] is a component for the other members pages (`/guilds`,
//!   `/upgrade`, `/admin/**`): the page wraps its content in it and keeps its own
//!   access rules.
//!
//! Link highlighting follows the request path; see [`link`].

mod card;
mod chrome;
mod guild_layout;
mod guilds;
pub mod link;
mod overview;
mod sidebar;
mod switcher;
mod upgrade;

pub use card::module_card;
pub use chrome::app_shell;
pub use guild_layout::GuildId;
use topcoat::router::RouterBuilder;

pub const GUILDS_TITLE: &str = "Servers - Zayden Dashboard";
pub const OVERVIEW_TITLE: &str = "Modules - Zayden Dashboard";
pub const UPGRADE_TITLE: &str = "Upgrade - Zayden Dashboard";

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.layout(guild_layout::guild_shell)
        .page(guilds::guilds)
        .page(overview::guild_overview)
        .page(overview::toggle_module)
        .page(upgrade::upgrade)
        .page(upgrade::link_kofi)
}
