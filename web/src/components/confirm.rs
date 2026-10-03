#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub async fn confirm_button(
    label: &str,
    prompt: &str,
    confirm: &str,
    #[default("btn btn-danger")] class: &str,
) -> Result<impl View> {
    Ok(view! {
        <details class="confirm">
            <summary class=(class)>
                <span class="confirm-label">(label)</span>
                <span class="confirm-cancel">"Cancel"</span>
            </summary>
            <div class="confirm-panel">
                <p class="confirm-prompt">(prompt)</p>
                <button type="submit" class="btn btn-danger">(confirm)</button>
            </div>
        </details>
    })
}
