#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{Child, View, component, view};

#[must_use]
pub fn describedby(id: &str, has_help: bool, has_error: bool) -> Option<String> {
    let ids: Vec<String> = [(has_help, "help"), (has_error, "error")]
        .into_iter()
        .filter(|(present, _)| *present)
        .map(|(_, suffix)| format!("{id}-{suffix}"))
        .collect();

    (!ids.is_empty()).then(|| ids.join(" "))
}

#[component]
pub async fn field_row(
    id: &str,
    label: &str,
    #[default] help: Option<&str>,
    #[default] error: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="field-row" data-invalid=(error.map(|_| ""))>
            <label class="field-label" for=(id)>(label)</label>
            (child)
            if let Some(help) = help {
                <p class="field-help" id=(format!("{id}-help"))>(help)</p>
            }
            if let Some(error) = error {
                <p class="field-error" id=(format!("{id}-error"))>(error)</p>
            }
        </div>
    })
}
