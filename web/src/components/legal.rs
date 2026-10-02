use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub(crate) async fn legal_links() -> Result<impl View> {
    Ok(view! {
        <nav class="legal-links" aria-label="Legal">
            <a href="/privacy">"Privacy Policy"</a>
            <a href="/terms">"Terms of Service"</a>
        </nav>
    })
}
