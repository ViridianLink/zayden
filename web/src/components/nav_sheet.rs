#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};

use super::icons::{Icon, icon};
use super::legal::legal_links;
use super::nav_links::{NavAccess, nav_links};

pub const SHEET_ID: &str = "nav-sheet";

/// The same navigation as the rail, in a full-height popover sheet opened by
/// the top bar's menu button. Esc, an outside click and the close button
/// dismiss it.
#[component]
pub async fn nav_sheet(
    access: NavAccess,
    #[default] guild_id: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <div id=(SHEET_ID) popover="" class="sheet-panel" aria-label="Menu">
            <div class="sheet-head">
                <span class="label">"Menu"</span>
                <button
                    type="button"
                    class="topbar-button"
                    popovertarget=(SHEET_ID)
                    popovertargetaction="hide"
                    aria-label="Close menu"
                    autofocus=""
                >
                    icon(name: Icon::X)
                </button>
            </div>
            <div class="sheet-body">
                nav_links(prefix: "sheet", access: access, guild_id: guild_id)
                <div class="nav-spacer"></div>
                legal_links()
            </div>
        </div>
    }
    .boxed())
}
