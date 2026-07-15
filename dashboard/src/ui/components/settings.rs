use leptos::prelude::*;

pub(crate) fn save_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! { <p class="success">"Saved."</p> }.into_any(),
        Err(ref e) if e.to_string().contains("pro_required") => view! {
            <p class="error">"A Pro subscription is required to save settings."</p>
        }
        .into_any(),
        Err(e) => view! { <p class="error">"Failed to save: " {e.to_string()}</p> }
            .into_any(),
    }
}

#[component]
pub(crate) fn SaveButton(is_pro: bool) -> impl IntoView {
    view! {
        <div class="form-actions">
            <button type="submit" class="btn btn-primary" disabled=move || !is_pro>
                {if is_pro { "Save" } else { "Pro Required" }}
            </button>
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
