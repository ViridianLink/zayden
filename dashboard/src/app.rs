use leptos::prelude::*;
use leptos_meta::{Stylesheet, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::ui::pages::guild_settings::GuildSettingsPage;
use crate::ui::pages::guilds::GuildListPage;
use crate::ui::pages::landing::LandingPage;
use crate::ui::pages::login::LoginPage;
use crate::ui::pages::modules::GuildOverviewPage;
use crate::ui::pages::not_found::NotFound;
use crate::ui::pages::upgrade::UpgradePage;

#[derive(Clone)]
pub struct UpgradeUrl(pub Option<String>);

#[cfg(feature = "ssr")]
#[must_use]
pub fn shell(options: LeptosOptions) -> impl IntoView {
    use leptos::hydration::{AutoReload, HydrationScripts};
    use leptos_meta::MetaTags;

    view! {
        <!DOCTYPE html>
        <html lang="en" attr:data-bot="zayden">
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

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/dashboard.css"/>
        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <Route path=path!("/") view=LandingPage/>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/upgrade") view=UpgradePage/>
                <Route path=path!("/guilds") view=GuildListPage/>
                <Route path=path!("/guild/:id") view=GuildOverviewPage/>
                <Route path=path!("/guild/:id/settings") view=GuildSettingsPage/>
            </Routes>
        </Router>
    }
}
