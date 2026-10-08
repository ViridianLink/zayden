use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::page;
use topcoat::router::request::uri;
use topcoat::view::{View, component, view};
use url::form_urlencoded;

use crate::auth;
use crate::components::brand::brand_mark;
use crate::components::legal::legal_links;

const SIGNED_IN_PATH: &str = "/guilds";

const AUTH_FAILED: &str = "auth_failed";

const AUTH_FAILED_TEXT: &str = "Signing in with Discord didn't finish, so you \
                                aren't signed in. Please try again.";

fn sign_in_failed(cx: &Cx) -> bool {
    form_urlencoded::parse(uri(cx).query().unwrap_or_default().as_bytes())
        .any(|(key, value)| key == "error" && value == AUTH_FAILED)
}

#[page("/login")]
pub(crate) async fn login(cx: &Cx) -> Result<impl View> {
    if matches!(auth::check_session(cx).await, Ok(true)) {
        return Err(see_other(SIGNED_IN_PATH).into());
    }

    Ok(view! { login_card(failed: sign_in_failed(cx)) })
}

#[component]
async fn login_card(failed: bool) -> Result<impl View> {
    Ok(view! {
        <main class="login-page" id="main" tabindex="-1">
            <div class="hero-glow"></div>
            <div class="login-card">
                <span class="brand">
                    brand_mark()
                    "Zayden"
                </span>
                <h1>"Welcome back"</h1>
                <p>"Connect your Discord account to manage your server settings."</p>
                if failed {
                    <p class="error" role="alert">(AUTH_FAILED_TEXT)</p>
                }
                <a href="/auth/discord" rel="external" class="btn btn-primary btn-lg">
                    (if failed { "Try again with Discord" } else { "Sign in with Discord" })
                </a>
            </div>
            legal_links()
        </main>
    })
}
