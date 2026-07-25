use leptos::prelude::*;

use super::icons::Icon;

pub(crate) fn save_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! { <p class="success">"Saved."</p> }.into_any(),
        Err(e) => view! { <p class="error">"Failed to save: " {e.to_string()}</p> }
            .into_any(),
    }
}

pub(crate) fn create_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! { <p class="success">"Creator channel created."</p> }.into_any(),
        Err(e) => {
            view! { <p class="error">"Failed to create channel: " {e.to_string()}</p> }
                .into_any()
        },
    }
}

#[component]
pub(crate) fn SaveButton() -> impl IntoView {
    view! {
        <div class="form-actions">
            <button type="submit" class="btn btn-primary">"Save"</button>
        </div>
    }
}

#[component]
pub(crate) fn ToggleField(
    label: &'static str,
    name: &'static str,
    value: bool,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <div class="select">
                <select name=name>
                    <option value="true" selected=value>"Enabled"</option>
                    <option value="false" selected=!value>"Disabled"</option>
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
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <input
                type="text"
                name=name
                value=value
                placeholder="(not set)"
                pattern="[0-9]*"
            />
        </div>
    }
}
