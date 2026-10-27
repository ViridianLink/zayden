mod endpoints;
mod index;
mod page;
mod view;

pub use endpoints::{NewEmoji, Reply};
pub use index::{CatalogIndex, draft_key, editor_id, emoji_keys, missing_emojis};
pub use page::editor_data;
pub(crate) use page::missing_loadout;
use topcoat::asset::{Asset, asset};
use topcoat::router::{path_param, segment};

pub const EDITOR_TITLE: &str = "Edit loadout - Zayden Dashboard";
pub const NEW_LOADOUT_TITLE: &str = "New loadout - Zayden Dashboard";
pub const LOADOUT_NOT_FOUND_TITLE: &str = "Loadout not found - Zayden Dashboard";
pub const LOADOUT_EDITOR_JS: Asset =
    asset!("../../../../../../assets/loadout-editor.js");

segment!(kind = Group);

path_param!(pub loadout_id);
