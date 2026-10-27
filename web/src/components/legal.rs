use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, component, view};

#[component]
pub(crate) async fn legal_links(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <nav class="legal-links" aria-label="Legal">
            <a (link_attrs(cx, "/privacy", prefetch_mode(cx)))>"Privacy Policy"</a>
            <a (link_attrs(cx, "/terms", prefetch_mode(cx)))>"Terms of Service"</a>
        </nav>
    })
}
