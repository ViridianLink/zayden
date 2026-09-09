use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::{use_location, use_params_map};

use super::icons::Icon;
use super::server_switcher::ServerSwitcher;
use super::skeleton::Skeleton;
use super::tier_badge::TierBadge;
use crate::server::auth::check_session;
use crate::server::operator::{guild_operator_access, is_operator};
use crate::ui::nav::MODULES;

#[derive(Clone, Copy)]
pub struct ModulesOpen(pub RwSignal<bool>);

#[component]
fn Frame(sidebar: AnyView, children: Children) -> impl IntoView {
    view! {
        <div class="app">
            <AppNavBar/>
            <div class="app-body">
                {sidebar}
                <main class="app-main">
                    {children()}
                </main>
            </div>
        </div>
    }
}

#[component]
pub(crate) fn AppShell(children: Children) -> impl IntoView {
    view! {
        <Frame sidebar=view! { <TopSidebar/> }.into_any()>
            {children()}
        </Frame>
    }
}

#[component]
pub(crate) fn GuildShell() -> impl IntoView {
    let params = use_params_map();
    let guild_id =
        Signal::derive(move || params.with(|p| p.get("id").unwrap_or_default()));

    view! {
        <Frame sidebar=view! { <GuildSidebar guild_id/> }.into_any()>
            <Outlet/>
        </Frame>
    }
}

#[component]
fn AppNavBar() -> AnyView {
    let session = Resource::new_blocking(|| (), |()| check_session());

    view! {
        <nav class="app-navbar">
            <A href="/guilds" attr:class="brand">
                <span class="brand-mark">"Z"</span>
                "Zayden"
            </A>
            <div class="app-navbar-links">
                <TierBadge/>
                <Suspense fallback=|| view! {
                    <Skeleton class="skeleton-btn"/>
                }>
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
    .into_any()
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
fn GuildSidebar(guild_id: Signal<String>) -> impl IntoView {
    view! {
        <aside class="app-sidebar">
            <ServerSwitcher guild_id/>
            <OperatorBadge guild_id/>
            <div class="app-sidebar-heading">"Manage"</div>
            <ModulesGroup guild_id/>
            <div class="app-sidebar-spacer"></div>
            <SidebarLink href="/guilds".to_string() icon="server" label="All servers" exact=true/>
            <OperatorLink/>
            <SidebarLink href="/upgrade".to_string() icon="zap" label="Upgrade to Pro"/>
        </aside>
    }
}

#[component]
fn OperatorBadge(guild_id: Signal<String>) -> impl IntoView {
    let access =
        Resource::new_blocking(move || guild_id.get(), guild_operator_access);

    view! {
        <Suspense fallback=|| ()>
            {move || access.get()
                .and_then(Result::ok)
                .unwrap_or(false)
                .then(|| view! {
                    <div class="operator-badge">
                        <Icon name="shield"/>
                        <span>"Operator access"</span>
                    </div>
                })}
        </Suspense>
    }
}

#[component]
fn OperatorLink() -> impl IntoView {
    let operator = Resource::new_blocking(|| (), |()| is_operator());

    view! {
        <Suspense fallback=|| ()>
            {move || operator.get()
                .and_then(Result::ok)
                .unwrap_or(false)
                .then(|| view! {
                    <SidebarLink
                        href="/admin/servers".to_string()
                        icon="shield"
                        label="All bot servers"
                        exact=true
                    />
                })}
        </Suspense>
    }
}

#[component]
fn ModulesGroup(guild_id: Signal<String>) -> impl IntoView {
    let location = use_location();
    let open = use_context::<ModulesOpen>()
        .map_or_else(|| RwSignal::new(true), |ctx| ctx.0);

    let overview_href = move || format!("/guild/{}", guild_id.get());
    let overview_class = move || {
        if location.pathname.get() == overview_href() {
            "app-sidebar-link active"
        } else {
            "app-sidebar-link"
        }
    };

    let current = Memo::new(move |_| {
        let path = location.pathname.get();
        let gid = guild_id.get();
        if path == format!("/guild/{gid}/settings") {
            format!("/guild/{gid}/settings/general")
        } else {
            path
        }
    });

    let caret_class = move || {
        if open.get() { "app-sidebar-caret open" } else { "app-sidebar-caret" }
    };

    let sublist = move || {
        open.get().then(|| {
            let gid = guild_id.get();
            let links = MODULES
                .iter()
                .map(|module| {
                    let href = module.href(&gid);
                    let target = href.clone();
                    let class = move || {
                        if current.get() == target {
                            "app-sidebar-sublink active"
                        } else {
                            "app-sidebar-sublink"
                        }
                    };

                    view! { <A href=href attr:class=class>{module.label}</A> }
                })
                .collect_view();

            view! { <div class="app-sidebar-sublist">{links}</div> }
        })
    };

    view! {
        <div class="app-sidebar-group">
            <div class="app-sidebar-group-head">
                <A href=overview_href attr:class=overview_class>
                    <Icon name="grid"/>
                    <span>"Modules"</span>
                </A>
                <button
                    type="button"
                    class=caret_class
                    aria-label="Toggle module list"
                    aria-expanded=move || open.get().to_string()
                    on:click=move |_| open.update(|v| *v = !*v)
                >
                    <Icon name="chevron-down"/>
                </button>
            </div>
            {sublist}
        </div>
    }
}

#[component]
fn TopSidebar() -> impl IntoView {
    view! {
        <aside class="app-sidebar">
            <div class="app-sidebar-heading">"Dashboard"</div>
            <SidebarLink href="/guilds".to_string() icon="server" label="Servers" exact=true/>
            <OperatorLink/>
            <SidebarLink href="/upgrade".to_string() icon="zap" label="Upgrade to Pro"/>
        </aside>
    }
}
