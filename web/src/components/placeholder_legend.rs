#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub async fn placeholder_legend(entries: &[(&str, &str)]) -> Result<impl View> {
    Ok(view! {
        <ul class="placeholder-legend">
            #[key(*token)]
            for (token, description) in entries {
                <li>
                    <code>(*token)</code>
                    (format!(" - {description}"))
                </li>
            }
        </ul>
    })
}
