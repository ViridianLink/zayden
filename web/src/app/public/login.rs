use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::page;
use topcoat::router::request::uri;
use topcoat::view::{View, attributes, class, component, view};
use url::form_urlencoded;

use super::layout::public_layout;
use crate::auth;
use crate::components::alert::{AlertVariant, alert, alert_title};
use crate::components::brand::brand_mark;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};

const SIGNED_IN_PATH: &str = "/guilds";

const AUTH_FAILED: &str = "auth_failed";

const AUTH_FAILED_TEXT: &str = "Signing in with Discord didn't finish, so you \
                                aren't signed in. Please try again.";

fn sign_in_failed(cx: &Cx) -> bool {
    form_urlencoded::parse(uri(cx).query().unwrap_or_default().as_bytes())
        .any(|(key, value)| key == "error" && value == AUTH_FAILED)
}

#[page]
pub(crate) async fn login(cx: &Cx) -> Result<impl View> {
    if matches!(auth::check_session(cx).await, Ok(true)) {
        return Err(see_other(SIGNED_IN_PATH).into());
    }

    Ok(view! { login_card(failed: sign_in_failed(cx)) })
}

#[component]
async fn login_card(failed: bool) -> Result<impl View> {
    Ok(view! {
        public_layout(
            <div class="login-page">
                <div class="hero-glow"></div>
                <div class="login-card">
                    <span class="brand">
                        brand_mark()
                        "Zayden"
                    </span>
                    <h1>"Welcome back"</h1>
                    <p>
                        "Connect your Discord account to manage your server settings."
                    </p>
                    if failed {
                        alert(
                            variant: AlertVariant::Destructive,
                            attrs: attributes! { class="error" role="alert" },
                            alert_title((AUTH_FAILED_TEXT))
                        )
                    }
                    <a
                        href="/auth/discord"
                        rel="external"
                        class=(class!(
                            button_variants(ButtonVariant::Primary, ButtonSize::Lg),
                            "w-full",
                        ))
                    >
                        (if failed {
                            "Try again with Discord"
                        } else {
                            "Sign in with Discord"
                        })
                    </a>
                </div>
            </div>
        )
    })
}
