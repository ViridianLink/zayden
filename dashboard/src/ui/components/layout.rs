use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_location, use_params_map};

use super::icons::Icon;
use super::server_switcher::ServerSwitcher;
use super::tier_badge::TierBadge;
use crate::server::auth::check_session;

#[component]
pub(crate) fn AppShell(children: Children) -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id"));

    view! {
        <div class="app">
            <AppNavBar/>
            <div class="app-body">
                {move || guild_id().map_or_else(
                    || view! { <TopSidebar/> }.into_any(),
                    |id| view! { <GuildSidebar guild_id=id/> }.into_any(),
                )}
                <main class="app-main">
                    {children()}
                </main>
            </div>
        </div>
    }
}

#[component]
fn AppNavBar() -> impl IntoView {
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <nav class="app-navbar">
            <A href="/guilds" attr:class="brand">
                <span class="brand-mark">"Z"</span>
                "Zayden"
            </A>
            <div class="app-navbar-links">
                <TierBadge/>
                <Suspense fallback=|| ()>
                    {move || {
                        session.get().and_then(Result::ok).map(|logged_in| {
                            if logged_in {
                                view! {
                                    <a href="/logout" rel="external" class="btn btn-ghost">
                                        <Icon name="log-out"/>
                                        "Log out"
                                    </a>
                                }
                                .into_any()
                            } else {
                                view! {
                                    <a href="/auth/discord" rel="external" class="btn btn-primary">
                                        "Sign in"
                                    </a>
                                }
                                .into_any()
                            }
                        })
                    }}
                </Suspense>
            </div>
        </nav>
    }
}

#[component]
fn SidebarLink(
    href: String,
    icon: &'static str,
    #[prop(into)] label: String,
    #[prop(default = false)] exact: bool,
) -> impl IntoView {
    let location = use_location();
    let target = href.clone();
    let class = move || {
        let path = location.pathname.get();
        let active = if exact { path == target } else { path.starts_with(&target) };
        if active { "app-sidebar-link active" } else { "app-sidebar-link" }
    };

    view! {
        <A href=href attr:class=class>
            <Icon name=icon/>
            <span>{label}</span>
        </A>
    }
}

#[component]
fn GuildSidebar(guild_id: String) -> impl IntoView {
    let overview_href = format!("/guild/{guild_id}");
    let settings_href = format!("/guild/{guild_id}/settings");

    view! {
        <aside class="app-sidebar">
            <ServerSwitcher guild_id=guild_id/>
            <div class="app-sidebar-heading">"Manage"</div>
            <SidebarLink href=overview_href icon="grid" label="Modules" exact=true/>
            <SidebarLink href=settings_href icon="settings" label="Settings"/>
            <div class="app-sidebar-spacer"></div>
            <SidebarLink href="/guilds".to_string() icon="server" label="All servers" exact=true/>
            <SidebarLink href="/upgrade".to_string() icon="zap" label="Upgrade to Pro"/>
        </aside>
    }
}

#[component]
fn TopSidebar() -> impl IntoView {
    view! {
        <aside class="app-sidebar">
            <div class="app-sidebar-heading">"Dashboard"</div>
            <SidebarLink href="/guilds".to_string() icon="server" label="Servers" exact=true/>
            <SidebarLink href="/upgrade".to_string() icon="zap" label="Upgrade to Pro"/>
        </aside>
    }
}
