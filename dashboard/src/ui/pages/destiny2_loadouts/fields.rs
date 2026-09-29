use leptos::prelude::*;

#[component]
pub(crate) fn TextInput(
    #[prop(into)] id: String,
    label: &'static str,
    value: RwSignal<String>,
    #[prop(default = "")] placeholder: &'static str,
    #[prop(default = "text")] kind: &'static str,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label for=id.clone()>{label}</label>
            <input
                id=id
                class="input"
                type=kind
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </div>
    }
}

#[component]
pub(crate) fn OptionSelect(
    #[prop(into)] id: String,
    label: &'static str,
    value: RwSignal<String>,
    options: Vec<String>,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label for=id.clone()>{label}</label>
            <select
                id=id
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
