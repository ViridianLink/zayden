use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::page;
use topcoat::view::{View, ViewExt, component, view};

use crate::auth;
use crate::components::legal::legal_links;

const SIGNED_IN_PATH: &str = "/guilds";

#[page("/login")]
pub(crate) async fn login(cx: &Cx) -> Result<impl View> {
    Ok(match auth::check_session(cx).await {
        Ok(true) => return Err(see_other(SIGNED_IN_PATH).into()),
        Ok(false) => view! { login_card() }.boxed(),
        Err(_) => view! {}.boxed(),
    })
}

#[component]
async fn login_card() -> Result<impl View> {
    Ok(view! {
        <div class="login-page">
            <div class="hero-glow"></div>
            <div class="login-card">
                <span class="brand">
                    <span class="brand-mark">"Z"</span>
                    "Zayden"
                </span>
                <h1>"Welcome back"</h1>
                <p>"Connect your Discord account to manage your server settings."</p>
                <a href="/auth/discord" rel="external" class="btn btn-primary btn-lg">
                    "Sign in with Discord"
                </a>
            </div>
            legal_links()
        </div>
    })
}
