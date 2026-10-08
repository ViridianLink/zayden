use topcoat::Result;
use topcoat::router::not_found;
use topcoat::view::{View, component, view};

use crate::components::brand::brand_mark;
use crate::components::legal::legal_links;

not_found!("/");

#[component]
pub(crate) async fn not_found_page() -> Result<impl View> {
    Ok(view! {
        <div class="login-page" id="main" tabindex="-1">
            <div class="hero-glow"></div>
            <div class="login-card">
                <span class="brand">
                    brand_mark()
                    "Zayden"
                </span>
                <h1>"404"</h1>
                <p>"We couldn't find that page."</p>
                <a href="/" class="btn btn-primary btn-lg">"Back home"</a>
            </div>
            legal_links()
        </div>
    })
}
