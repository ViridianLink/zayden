use leptos::prelude::*;

use super::icons::Icon;

#[component]
pub(crate) fn Alert(
    #[prop(into)] class: String,
    role: &'static str,
    #[prop(into)] message: String,
) -> impl IntoView {
    let dismissed = RwSignal::new(false);

    view! {
        <div class=class role=role hidden=move || dismissed.get()>
            <span>{message}</span>
            <button
                type="button"
                class="alert-dismiss"
                aria-label="Dismiss"
                on:click=move |_| dismissed.set(true)
            >
                <Icon name="x"/>
            </button>
        </div>
    }
}

pub(crate) fn save_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! {
            <Alert class="alert success" role="status" message="Saved."/>
        }
        .into_any(),
        Err(e) => view! {
            <Alert
                class="alert error"
                role="alert"
                message=format!("Failed to save: {e}")
            />
        }
        .into_any(),
    }
}

pub(crate) fn create_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! {
            <Alert
                class="alert success"
                role="status"
                message="Creator channel created."
            />
        }
        .into_any(),
        Err(e) => view! {
            <Alert
                class="alert error"
                role="alert"
                message=format!("Failed to create channel: {e}")
            />
        }
        .into_any(),
    }
}

#[component]
pub(crate) fn SaveButton(#[prop(into)] pending: Signal<bool>) -> impl IntoView {
    view! {
        <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled=pending>
                {move || if pending.get() { "Saving\u{2026}" } else { "Save" }}
            </button>
        </div>
    }
}

#[component]
pub(crate) fn ToggleField(
    label: &'static str,
    name: &'static str,
    value: bool,
    #[prop(default = "Enabled")] on_label: &'static str,
    #[prop(default = "Disabled")] off_label: &'static str,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <div class="select">
                <select class="input" name=name>
                    <option value="true" selected=value>{on_label}</option>
                    <option value="false" selected=!value>{off_label}</option>
                </select>
                <span class="select-chevron"><Icon name="chevron-down"/></span>
            </div>
        </div>
    }
}

#[component]
pub(crate) fn SettingField(
    label: &'static str,
    name: &'static str,
    value: String,
    #[prop(default = "[0-9]*")] pattern: &'static str,
    #[prop(default = "(not set)")] placeholder: &'static str,
    #[prop(optional, into)] hint: Option<&'static str>,
    #[prop(default = "text")] input_type: &'static str,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <input
                class="input"
                type=input_type
                name=name
                value=value
                placeholder=placeholder
                pattern=pattern
            />
            {hint.map(|hint| view! { <p class="field-hint">{hint}</p> })}
        </div>
    }
}
