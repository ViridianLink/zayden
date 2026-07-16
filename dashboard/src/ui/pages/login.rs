use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::Redirect;

use crate::server::auth::check_session;

#[component]
pub(crate) fn LoginPage() -> impl IntoView {
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <Title text="Sign In - Zayden Dashboard"/>

        <Suspense fallback=|| ()>
            {move || {
                session.get()
                    .and_then(Result::ok)
                    .filter(|&logged_in| logged_in)
                    .map(|_| view! { <Redirect path="/guilds"/> })
            }}
        </Suspense>
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
        </div>
    }
}
