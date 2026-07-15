use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::{A, Outlet};

use crate::server::auth::check_session;
use super::tier_badge::TierBadge;

#[component]
pub(crate) fn Layout() -> impl IntoView {
    view! {
        <div class="layout">
            <Title text="Zayden Dashboard"/>
            <div class="watermark" aria-hidden="true"></div>
            <NavBar/>
            <div class="layout-body">
                <Sidebar/>
                <main class="layout-main">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

#[component]
fn NavBar() -> impl IntoView {
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <nav class="navbar">
            <A href="/" attr:class="navbar-brand">"Zayden"</A>
            <div class="navbar-links">
                <A href="/guilds">"Servers"</A>
                <A href="/upgrade">"Upgrade"</A>
                <TierBadge/>
                <Suspense fallback=|| ()>
                    {move || {
                        session.get()
                            .and_then(Result::ok)
                            .filter(|&logged_in| logged_in)
                            .map(|_| view! { <a href="/logout" class="btn btn-ghost">"Log out"</a> })
                    }}
                </Suspense>
            </div>
        </nav>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let open = RwSignal::new(true);

    let aside_class = move || {
        if open.get() { "sidebar" } else { "sidebar sidebar-collapsed" }
    };

    view! {
        <aside class=aside_class>
            <button
                class="sidebar-toggle"
                on:click=move |_| open.update(|v| *v = !*v)
            >
                {move || if open.get() { "←" } else { "→" }}
            </button>
            <Show when=move || open.get()>
                <nav class="sidebar-nav">
                    <A href="/guilds">"Servers"</A>
                    <A href="/upgrade">"Upgrade to Pro"</A>
                </nav>
            </Show>
        </aside>
    }
}
