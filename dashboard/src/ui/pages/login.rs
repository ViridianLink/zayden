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
            <h1>"Sign in to Zayden Dashboard"</h1>
            <p>"Connect your Discord account to manage server settings."</p>
            <a href="/auth/discord" rel="external" class="btn btn-primary">"Sign in with Discord"</a>
        </div>
    }
}
