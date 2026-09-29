use leptos::prelude::*;

use super::icons::Icon;

#[component]
pub(crate) fn KeyListField(
    label: &'static str,
    keys: RwSignal<Vec<String>>,
    list: &'static str,
    max: usize,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let add = move || {
        let key = draft.get_untracked().trim().to_owned();
        if key.is_empty() {
            return;
        }
        keys.update(|k| {
            if k.len() < max {
                k.push(key);
            }
        });
        draft.set(String::new());
    };

    view! {
        <div class="setting-field">
            <label>{label}</label>
            <div class="chip-list">
                {move || keys.get().into_iter().enumerate().map(|(i, key)| view! {
                    <span class="chip">
                        <span class="chip-label">{key}</span>
                        <button
                            type="button"
                            class="chip-remove"
                            title="Remove"
                            on:click=move |_| keys.update(|k| {
                                if i < k.len() {
                                    k.remove(i);
                                }
                            })
                        >
                            <Icon name="x"/>
                        </button>
                    </span>
                }).collect_view()}
            </div>
            <div class="chip-add">
                <input
                    class="input"
                    list=list
                    prop:value=move || draft.get()
                    on:input=move |ev| draft.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            add();
                        }
                    }
                />
                <button
                    type="button"
                    class="btn btn-secondary"
                    disabled=move || keys.with(|k| k.len() >= max)
                    on:click=move |_| add()
                >
                    "Add"
                </button>
            </div>
        </div>
    }
}
