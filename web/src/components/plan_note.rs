#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, component, view};

use super::badge::{BadgeVariant, badge_variants};

pub const PLANS_PATH: &str = "/upgrade";

#[component]
pub async fn plan_tag(label: &str) -> Result<impl View> {
    Ok(view! { <span class=(badge_variants(BadgeVariant::Outline))>(label)</span> })
}

#[component]
pub async fn plan_note(cx: &Cx, tier: &str, text: &str) -> Result<impl View> {
    Ok(view! {
        <p class="flex flex-wrap items-center gap-2 m-0 text-[13px] text-text-2">
            plan_tag(label: tier)
            <span>(text)</span>
            <a
                class="inline-flex items-center min-h-(--target-min) max-md:min-h-(--row-h)"
                (link_attrs(cx, PLANS_PATH, prefetch_mode(cx)))
            >
                "See plans"
            </a>
        </p>
    })
}
