use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::not_found;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, class, component, view};

use crate::auth::check_session;
use crate::components::brand::brand_mark;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::public::layout::public_layout;

not_found!("/");

#[component]
pub(crate) async fn not_found_page(cx: &Cx) -> Result<impl View> {
    let signed_in = check_session(cx).await.unwrap_or(false);
    let (href, label) =
        if signed_in { ("/guilds", "Back to servers") } else { ("/", "Back home") };

    Ok(view! {
        public_layout(
            <div class="login-page">
                <div class="hero-glow"></div>
                <div class="login-card">
                    <span class="brand">
                        brand_mark()
                        "Zayden"
                    </span>
                    <h1>"404"</h1>
                    <p>"We couldn't find that page."</p>
                    <a
                        class=(class!(
                            button_variants(ButtonVariant::Primary, ButtonSize::Lg),
                            "w-full",
                        ))
                        (link_attrs(cx, href, prefetch_mode(cx)))
                    >
                        (label)
                    </a>
                </div>
            </div>
        )
    })
}
