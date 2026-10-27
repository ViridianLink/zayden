#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, attributes, component, view};

use super::button::{ButtonSize, ButtonVariant, button_variants};

#[component]
pub async fn empty_state(
    cx: &Cx,
    title: &str,
    text: &str,
    #[default] action: Option<&str>,
    #[default] href: Option<&str>,
    #[default] external: bool,
) -> Result<impl View> {
    Ok(view! {
        <div
            class="flex flex-col items-start gap-2 max-w-(--form-max) py-8 px-6 border border-dashed border-line rounded-panel"
        >
            <h2 class="m-0 text-[17px] font-semibold">(title)</h2>
            <p class="m-0 mb-2 text-text-2">(text)</p>
            if let (Some(action), Some(href)) = (action, href) {
                let link = if external {
                    attributes! { href=(href) rel="external" }
                } else {
                    link_attrs(cx, href, prefetch_mode(cx))
                };
                <a
                    class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                    (link)
                >
                    (action)
                </a>
            }
        </div>
    })
}
