mod card;
mod chrome;
pub mod link;

pub use card::module_card;
pub(crate) use chrome::frame;
pub use chrome::{account_name, app_shell};

pub const GUILDS_TITLE: &str = "Servers - Zayden Dashboard";
pub const OVERVIEW_TITLE: &str = "Overview - Zayden Dashboard";
pub const UPGRADE_TITLE: &str = "Plans - Zayden";
