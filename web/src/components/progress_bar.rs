#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

/// The navigation progress line under the top bar. The shared script starts it
/// after 150 ms on link clicks and form submits.
#[component]
pub async fn progress_bar() -> Result<impl View> {
    Ok(
        view! { <div class="nav-progress" data-nav-progress="" aria-hidden="true"></div> },
    )
}
