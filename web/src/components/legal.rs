use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, component, view};

#[component]
pub(crate) async fn legal_links(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <nav
            class="flex flex-wrap gap-y-1 gap-x-4 mt-4 p-3 border-t border-solid border-t-rule text-[13px]"
            aria-label="Legal"
        >
            <a
                class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                (link_attrs(cx, "/privacy", prefetch_mode(cx)))
            >
                "Privacy Policy"
            </a>
            <a
                class="inline-flex items-center min-h-(--target-min) text-text-2 hover:text-text max-md:min-h-[44px]"
                (link_attrs(cx, "/terms", prefetch_mode(cx)))
            >
                "Terms of Service"
            </a>
        </nav>
    })
}
