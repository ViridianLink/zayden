mod endpoints;
mod index;
mod page;
mod view;

pub use endpoints::{NewEmoji, Reply};
pub use index::{CatalogIndex, draft_key, editor_id, emoji_keys, missing_emojis};
pub use page::editor_data;
pub(crate) use page::missing_loadout;
use topcoat::asset::{Asset, asset};
use topcoat::router::{RouterBuilder, path_param};

pub const EDITOR_TITLE: &str = "Edit loadout - Zayden Dashboard";
pub const NEW_LOADOUT_TITLE: &str = "New loadout - Zayden Dashboard";
pub const LOADOUT_NOT_FOUND_TITLE: &str = "Loadout not found - Zayden Dashboard";
pub const LOADOUT_EDITOR_JS: Asset = asset!("../../../assets/loadout-editor.js");

path_param!(pub loadout_id);

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(page::new_loadout)
        .page(page::edit_loadout)
        .route(endpoints::check)
        .route(endpoints::save)
        .route(endpoints::emoji)
}
