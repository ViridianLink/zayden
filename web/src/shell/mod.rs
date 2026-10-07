mod card;
mod chrome;
mod guild_layout;
mod guilds;
pub mod link;
mod overview;
mod upgrade;

pub use card::module_card;
pub use chrome::{account_name, app_shell};
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
