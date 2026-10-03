use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::runtime::signal;
use topcoat::view::{View, component, view};

use super::link::{aria_current, module_list_location, sidebar_active};
use super::switcher::server_switcher;
use crate::admin::{guild_operator_access, is_admin, is_operator};
use crate::components::icons::{Icon, icon};
use crate::components::legal::legal_links;
use crate::guild::dto::GuildInfo;
use crate::nav::MODULES;

const LINK: &str = "app-sidebar-link";
const LINK_ACTIVE: &str = "app-sidebar-link active";
const SUBLINK: &str = "app-sidebar-sublink";
const SUBLINK_ACTIVE: &str = "app-sidebar-sublink active";

#[component]
pub(super) async fn top_sidebar(cx: &Cx) -> Result<impl View> {
    let location = uri(cx).path();

    Ok(view! {
        <aside class="app-sidebar">
            <div class="app-sidebar-heading">"Dashboard"</div>
            sidebar_link(
                location: location,
                href: "/guilds",
                icon_name: Icon::Server,
                label: "Servers",
                exact: true
            )
            operator_link(location: location)
            admin_link(location: location)
            sidebar_link(
                location: location,
                href: "/upgrade",
                icon_name: Icon::Zap,
                label: "Upgrade to Pro"
            )
            legal_links()
        </aside>
    })
}

#[component]
pub(super) async fn guild_sidebar(
    cx: &Cx,
    guild_id: &str,
    guilds: &[GuildInfo],
) -> Result<impl View> {
    let location = uri(cx).path();

    Ok(view! {
        <aside class="app-sidebar">
            server_switcher(guild_id: guild_id, guilds: guilds)
            operator_badge(guild_id: guild_id)
            <div class="app-sidebar-heading">"Manage"</div>
            modules_group(guild_id: guild_id, location: location)
            <div class="app-sidebar-spacer"></div>
            sidebar_link(
                location: location,
                href: "/guilds",
                icon_name: Icon::Server,
                label: "All servers",
                exact: true
            )
            operator_link(location: location)
            admin_link(location: location)
            sidebar_link(
                location: location,
                href: "/upgrade",
                icon_name: Icon::Zap,
                label: "Upgrade to Pro"
            )
            legal_links()
        </aside>
    })
}

#[component]
async fn sidebar_link(
    location: &str,
    href: &str,
    icon_name: Icon,
    label: &str,
    #[default] exact: bool,
) -> Result<impl View> {
    let class =
        if sidebar_active(href, location, exact) { LINK_ACTIVE } else { LINK };

    Ok(view! {
        <a href=(href) aria-current=(aria_current(href, location)) class=(class)>
            icon(name: icon_name)
            <span>(label)</span>
        </a>
    })
}

#[component]
async fn operator_link(cx: &Cx, location: &str) -> Result<impl View> {
    let allowed = is_operator(cx).await.unwrap_or(false);

    Ok(view! {
        if allowed {
            sidebar_link(
                location: location,
                href: "/admin/servers",
                icon_name: Icon::Shield,
                label: "All bot servers",
                exact: true
            )
        }
    })
}

#[component]
async fn admin_link(cx: &Cx, location: &str) -> Result<impl View> {
    let allowed = is_admin(cx).await.unwrap_or(false);

    Ok(view! {
        if allowed {
            sidebar_link(
                location: location,
                href: "/admin/destiny2/loadouts",
                icon_name: Icon::Gamepad,
                label: "Loadout builder"
            )
        }
    })
}

#[component]
async fn operator_badge(cx: &Cx, guild_id: &str) -> Result<impl View> {
    let operator = guild_operator_access(cx, guild_id).await.unwrap_or(false);

    Ok(view! {
        if operator {
            <div class="operator-badge">
                icon(name: Icon::Shield)
                <span>"Operator access"</span>
            </div>
        }
    })
}

/// The `Modules` entry and its module list. The list starts open on every
/// page load; the caret folds it in the browser.
#[component]
async fn modules_group(
    cx: &Cx,
    guild_id: &str,
    location: &str,
) -> Result<impl View> {
    let open = signal(cx, || true);
    let overview = format!("/guild/{guild_id}");
    let overview_class = if location == overview { LINK_ACTIVE } else { LINK };
    let current = module_list_location(guild_id, location);

    Ok(view! {
        <div class="app-sidebar-group">
            <div class="app-sidebar-group-head">
                <a
                    href=(overview.as_str())
                    aria-current=(aria_current(&overview, location))
                    class=(overview_class)
                >
                    icon(name: Icon::Grid)
                    <span>"Modules"</span>
                </a>
                <button
                    type="button"
                    :class=$(if open.get() {
                        "app-sidebar-caret open"
                    } else {
                        "app-sidebar-caret"
                    })
                    aria-label="Toggle module list"
                    :aria-expanded=$(if open.get() { "true" } else { "false" })
                    @click=$(|_e| open.toggle())
                >
                    icon(name: Icon::ChevronDown)
                </button>
            </div>
            <div
                :class=$(if open.get() {
                    "app-sidebar-sublist open"
                } else {
                    "app-sidebar-sublist"
                })
            >
                #[key(module.label)]
                for module in MODULES {
                    let href = module.href(guild_id);
                    let class = if current == href { SUBLINK_ACTIVE } else { SUBLINK };
                    <a
                        href=(href.as_str())
                        aria-current=(aria_current(&href, location))
                        class=(class)
                    >
                        (module.label)
                    </a>
                }
            </div>
        </div>
    })
}
