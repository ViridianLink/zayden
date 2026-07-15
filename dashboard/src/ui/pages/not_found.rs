use leptos::prelude::*;

#[component]
pub(crate) fn NotFound() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"404"</h1>
            <p>"Page not found."</p>
        </div>
    }
}
