use leptos::prelude::*;
use leptos_meta::{Title, provide_meta_context};
use leptos_router::components::{A, Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::path;

/// HTML document shell — wraps the hydrated app in a full HTML page.
///
/// Only compiled in SSR mode. Serves as the Leptos page template and is used
/// both by `leptos_routes_with_context` (for SSR) and `file_and_error_handler`
/// (for unmatched paths / 404 fallback).
#[cfg(feature = "ssr")]
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos::hydration::{AutoReload, HydrationScripts};
    use leptos_meta::MetaTags;

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[expect(
    clippy::must_use_candidate,
    reason = "Leptos component; return value is consumed by the view system"
)]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/login") view=LoginPage/>
                <ParentRoute path=path!("/") view=Layout>
                    <Route path=path!("") view=Home/>
                    <Route path=path!("guilds") view=GuildListPage/>
                    <Route path=path!("guild/:id/settings") view=GuildSettingsPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// Top nav-bar + collapsible sidebar wrapping the active child route via
/// `<Outlet/>`.
#[component]
fn Layout() -> impl IntoView {
    view! {
        <div class="layout">
            <Title text="Zayden Dashboard"/>
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
    view! {
        <nav class="navbar">
            <A href="/" attr:class="navbar-brand">"Zayden"</A>
            <div class="navbar-links">
                <A href="/guilds">"Servers"</A>
            </div>
        </nav>
    }
}

#[component]
fn Sidebar() -> impl IntoView {
    let open = RwSignal::new(true);

    view! {
        <aside class="sidebar">
            <button
                class="sidebar-toggle"
                on:click=move |_| open.update(|v| *v = !*v)
            >
                {move || if open.get() { "←" } else { "→" }}
            </button>
            <Show when=move || open.get()>
                <nav class="sidebar-nav">
                    <A href="/guilds">"Servers"</A>
                </nav>
            </Show>
        </aside>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Dashboard"</h1>
            <p>"Select a server from the sidebar to manage its settings."</p>
        </div>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    view! {
        <div class="page login-page">
            <h1>"Sign in"</h1>
            <p>"Sign in with Discord to continue."</p>
        </div>
    }
}

#[component]
fn GuildListPage() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Your Servers"</h1>
            <p>"Loading servers\u{2026}"</p>
        </div>
    }
}

#[component]
fn GuildSettingsPage() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"Server Settings"</h1>
            <p>"Loading settings\u{2026}"</p>
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="page">
            <h1>"404"</h1>
            <p>"Page not found."</p>
        </div>
    }
}
