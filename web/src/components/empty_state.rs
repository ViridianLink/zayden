#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub async fn empty_state(
    title: &str,
    text: &str,
    #[default] action: Option<&str>,
    #[default] href: Option<&str>,
    #[default] external: bool,
) -> Result<impl View> {
    let rel = external.then_some("external");

    Ok(view! {
        <div class="empty-state">
            <h2 class="empty-title">(title)</h2>
            <p class="empty-text">(text)</p>
            if let (Some(action), Some(href)) = (action, href) {
                <a href=(href) rel=(rel) class="btn btn-primary">(action)</a>
            }
        </div>
    })
}
