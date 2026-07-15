use leptos::prelude::*;

#[component]
pub(crate) fn Home() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Dashboard"</h1>
            <p>"Select a server from the sidebar to manage its settings."</p>
        </div>
    }
}
