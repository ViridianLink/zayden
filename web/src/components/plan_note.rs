#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

pub const PLANS_PATH: &str = "/upgrade";

/// The word "Pro" (or another tier) as text, for a locked option.
#[component]
pub async fn plan_tag(label: &str) -> Result<impl View> {
    Ok(view! { <span class="plan-tag">(label)</span> })
}

/// A limit written beside the control it affects: the tier, the reason and the
/// one "See plans" link.
#[component]
pub async fn plan_note(tier: &str, text: &str) -> Result<impl View> {
    Ok(view! {
        <p class="plan-note">
            plan_tag(label: tier)
            <span>(text)</span>
            <a href=(PLANS_PATH)>"See plans"</a>
        </p>
    })
}
