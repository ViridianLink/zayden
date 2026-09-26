use leptos::prelude::*;

use super::public_layout::PublicLayout;

const CONTACT_EMAIL: &str = "kilooscarsix@gmail.com";

#[component]
pub(crate) fn LegalDocument(
    title: &'static str,
    updated: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <PublicLayout>
            <main class="legal">
                <h1>{title}</h1>
                <p class="legal-updated">"Last updated: "{updated}</p>
                {children()}
            </main>
        </PublicLayout>
    }
}

#[component]
pub(crate) fn LegalLinks() -> impl IntoView {
    view! {
        <nav class="legal-links" aria-label="Legal">
            <a href="/privacy">"Privacy Policy"</a>
            <a href="/terms">"Terms of Service"</a>
        </nav>
    }
}

#[component]
pub(crate) fn ContactEmail() -> impl IntoView {
    view! { <strong>{CONTACT_EMAIL}</strong> }
}
