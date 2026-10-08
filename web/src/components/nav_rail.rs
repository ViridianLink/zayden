#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};

use super::legal::legal_links;
use super::nav_links::{NavAccess, nav_links};

#[component]
pub async fn nav_rail(
    access: NavAccess,
    #[default] guild_id: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <div class="rail">
            nav_links(prefix: "rail", access: access, guild_id: guild_id)
            <div class="nav-spacer"></div>
            legal_links()
        </div>
    }
    .boxed())
}
