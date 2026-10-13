use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::not_found;
use topcoat::view::{View, component, view};

use crate::auth::check_session;
use crate::components::brand::brand_mark;
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
                    <a href=(href) class="btn btn-primary btn-lg">(label)</a>
                </div>
            </div>
        )
    })
}
