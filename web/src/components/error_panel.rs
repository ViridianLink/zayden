#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, attributes, component, view};

use super::button::{ButtonSize, ButtonVariant, button_variants};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorAction {
    pub label: String,
    pub href: String,
    pub external: bool,
}

impl ErrorAction {
    #[must_use]
    pub fn new(label: &str, href: &str) -> Self {
        Self { label: label.to_owned(), href: href.to_owned(), external: false }
    }

    #[must_use]
    pub const fn external(mut self) -> Self {
        self.external = true;
        self
    }
}

#[component]
pub async fn error_panel(
    cx: &Cx,
    title: &str,
    message: &str,
    actions: &[ErrorAction],
    #[default] alert: bool,
) -> Result<impl View> {
    let role = alert.then_some("alert");

    Ok(view! {
        <section
            class="flex flex-col gap-3 max-w-(--form-max) p-6 border border-solid border-danger rounded-panel bg-surface-1"
            role=(role)
        >
            <h1 class="error-title">(title)</h1>
            <p class="m-0 text-text-2">(message)</p>
            <div class="flex flex-wrap gap-2">
                #[key(index)]
                for (index, action) in actions.iter().enumerate() {
                    let variant = if index == 0 {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    };
                    let link = if action.external {
                        attributes! { href=(action.href.as_str()) rel="external" }
                    } else {
                        link_attrs(cx, action.href.as_str(), prefetch_mode(cx))
                    };
                    <a class=(button_variants(variant, ButtonSize::Md)) (link)>
                        (action.label.as_str())
                    </a>
                }
            </div>
        </section>
    })
}
