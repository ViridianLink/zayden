#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use super::confirm_dialog::confirm_dialog;

#[component]
pub async fn confirm_button(
    id: &str,
    label: &str,
    prompt: &str,
    confirm: &str,
    #[default("btn btn-danger")] class: &str,
    #[default] object: Option<&str>,
) -> Result<impl View> {
    let title = object.map_or_else(
        || format!("{confirm}?"),
        |object| format!("{confirm} {object}?"),
    );

    Ok(view! {
        confirm_dialog(
            id: id,
            trigger: label,
            title: &title,
            description: Some(prompt),
            confirm: confirm,
            trigger_class: class
        )
    })
}
