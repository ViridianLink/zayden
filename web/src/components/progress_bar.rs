#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub async fn progress_bar() -> Result<impl View> {
    Ok(
        view! { <div class="nav-progress" data-nav-progress="" aria-hidden="true"></div> },
    )
}
