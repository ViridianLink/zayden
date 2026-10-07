#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

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
    title: &str,
    message: &str,
    actions: &[ErrorAction],
    #[default] alert: bool,
) -> Result<impl View> {
    let role = alert.then_some("alert");

    Ok(view! {
        <section class="error-panel" role=(role)>
            <h1 class="error-title">(title)</h1>
            <p class="error-text">(message)</p>
            <div class="error-actions">
                #[key(index)]
                for (index, action) in actions.iter().enumerate() {
                    let class = if index == 0 {
                        "btn btn-primary"
                    } else {
                        "btn btn-secondary"
                    };
                    let rel = action.external.then_some("external");
                    <a href=(action.href.as_str()) rel=(rel) class=(class)>
                        (action.label.as_str())
                    </a>
                }
            </div>
        </section>
    })
}
