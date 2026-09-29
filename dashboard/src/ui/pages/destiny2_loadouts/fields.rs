use leptos::prelude::*;

#[component]
pub(crate) fn TextInput(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(optional)] list: Option<&'static str>,
    #[prop(default = "")] placeholder: &'static str,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <input
                class="input"
                list=list
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </div>
    }
}

#[component]
pub(crate) fn OptionSelect(
    label: &'static str,
    value: RwSignal<String>,
    options: Vec<String>,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <select
                class="input"
                prop:value=move || value.get()
                on:change=move |ev| value.set(event_target_value(&ev))
            >
                {options
                    .into_iter()
                    .map(|o| {
                        let selected = value.with_untracked(|v| *v == o);
                        let text = o.clone();
                        view! { <option value=o selected=selected>{text}</option> }
                    })
                    .collect_view()}
            </select>
        </div>
    }
}

#[component]
pub(crate) fn Datalist(id: &'static str, values: Vec<String>) -> impl IntoView {
    view! {
        <datalist id=id>
            {values.into_iter().map(|v| view! { <option value=v/> }).collect_view()}
        </datalist>
    }
}
