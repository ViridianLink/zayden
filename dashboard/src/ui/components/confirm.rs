use leptos::prelude::*;

#[component]
pub(crate) fn ConfirmButton(
    label: &'static str,
    prompt: &'static str,
    confirm: &'static str,
    #[prop(into)] pending: Signal<bool>,
    #[prop(default = "btn btn-danger")] class: &'static str,
) -> impl IntoView {
    view! {
        <details class="confirm">
            <summary class=class>
                <span class="confirm-label">{label}</span>
                <span class="confirm-cancel">"Cancel"</span>
            </summary>
            <div class="confirm-panel">
                <p class="confirm-prompt">{prompt}</p>
                <button
                    type="submit"
                    class="btn btn-danger"
                    disabled=pending
                >
                    {confirm}
                </button>
            </div>
        </details>
    }
}
