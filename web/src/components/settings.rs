#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::signal;
use topcoat::view::{View, component, view};

use super::icons::{Icon, icon};
use crate::util::server_error_text;

/// A dismissible banner. `role` is `"status"` for confirmations and
/// `"alert"` for failures. Inside a `for` loop, key the loop; never render it
/// from a memoized helper, since its signal needs a distinct identity.
#[component]
pub async fn alert(
    cx: &Cx,
    class: &str,
    role: &str,
    message: &str,
) -> Result<impl View> {
    let dismissed = signal(cx, || false);

    Ok(view! {
        <div class=(class) role=(role) :hidden=$(dismissed.get())>
            <span>(message)</span>
            <button
                type="button"
                class="alert-dismiss"
                aria-label="Dismiss"
                @click=$(|_e| dismissed.set(true))
            >
                icon(name: Icon::X)
            </button>
        </div>
    })
}

/// `Err` carries the raw server message; it is shown as a server error.
#[component]
pub async fn save_feedback(outcome: Result<(), &str>) -> Result<impl View> {
    Ok(view! { feedback(outcome: outcome, ok: "Saved.", err: "Failed to save") })
}

/// `Err` carries the raw server message; it is shown as a server error.
#[component]
pub async fn create_feedback(outcome: Result<(), &str>) -> Result<impl View> {
    Ok(view! {
        feedback(
            outcome: outcome,
            ok: "Creator channel created.",
            err: "Failed to create channel"
        )
    })
}

/// `Err` carries the raw server message; it is shown as a server error.
#[component]
pub async fn delete_feedback(outcome: Result<(), &str>) -> Result<impl View> {
    Ok(view! {
        feedback(outcome: outcome, ok: "Loadout deleted.", err: "Failed to delete")
    })
}

#[component]
async fn feedback(
    outcome: Result<(), &str>,
    ok: &str,
    err: &str,
) -> Result<impl View> {
    let failure =
        outcome.err().map(|detail| format!("{err}: {}", server_error_text(detail)));

    Ok(view! {
        match failure.as_deref() {
            None => alert(class: "alert success", role: "status", message: ok),
            Some(message) => alert(
                class: "alert error",
                role: "alert",
                message: message
            ),
        }
    })
}

/// While its form submits, the button reads "Saving…" (`data-pending-label`).
/// Put it in a `<form method="post" data-pending="">`.
#[component]
pub async fn save_button() -> Result<impl View> {
    Ok(view! {
        <div class="form-actions">
            <button
                type="submit"
                class="btn btn-primary"
                data-pending-label="Saving\u{2026}"
            >
                "Save"
            </button>
        </div>
    })
}

#[component]
pub async fn toggle_field(
    label: &str,
    name: &str,
    value: bool,
    #[default("Enabled")] on_label: &str,
    #[default("Disabled")] off_label: &str,
) -> Result<impl View> {
    Ok(view! {
        <div class="setting-field">
            <label>(label)</label>
            <div class="select">
                <select class="input" name=(name)>
                    <option value="true" selected=(value)>(on_label)</option>
                    <option value="false" selected=(!value)>(off_label)</option>
                </select>
                <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
            </div>
        </div>
    })
}

/// A labelled text input. `pattern` is enforced by the browser on submit;
/// the default accepts digits only, for Discord ids.
#[component]
pub async fn setting_field(
    label: &str,
    name: &str,
    value: &str,
    #[default("[0-9]*")] pattern: &str,
    #[default("(not set)")] placeholder: &str,
    #[default] hint: Option<&str>,
    #[default("text")] input_type: &str,
) -> Result<impl View> {
    Ok(view! {
        <div class="setting-field">
            <label>(label)</label>
            <input
                class="input"
                type=(input_type)
                name=(name)
                value=(value)
                placeholder=(placeholder)
                pattern=(pattern)
            >
            if let Some(hint) = hint {
                <p class="field-hint">(hint)</p>
            }
        </div>
    })
}
