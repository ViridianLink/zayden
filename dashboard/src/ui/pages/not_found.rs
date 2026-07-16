use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub(crate) fn NotFound() -> impl IntoView {
    view! {
        <Title text="Not found - Zayden"/>
        <div class="login-page">
            <div class="hero-glow"></div>
            <div class="login-card">
                <span class="brand">
                    <span class="brand-mark">"Z"</span>
                    "Zayden"
                </span>
                <h1>"404"</h1>
                <p>"We couldn't find that page."</p>
                <a href="/" class="btn btn-primary btn-lg">"Back home"</a>
            </div>
        </div>
    }
}
