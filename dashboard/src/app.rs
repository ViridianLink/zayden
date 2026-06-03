use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_router::components::{Route, Router, Routes};
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

/// Root application component.
///
/// Placeholder — routes are added in M11.7.3.
#[expect(
    clippy::must_use_candidate,
    reason = "Leptos component; return value is consumed by the view system"
)]
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <main>
                <Routes fallback=|| view! { <p>"Page not found."</p> }>
                    <Route path=path!("/") view=|| ()/>
                </Routes>
            </main>
        </Router>
    }
}
