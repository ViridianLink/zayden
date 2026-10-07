#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, ViewExt, component, view};

use super::icons::{Icon, icon};
use crate::admin::{guild_operator_access, is_admin, is_operator};
use crate::nav::{GENERAL, GROUPS, ModuleNav};
use crate::shell::link::{is_active_for, module_list_location};

const OVERVIEW: &str = "Overview";
const ALL_SERVERS: &str = "Servers";
const PLANS: &str = "Plans";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavAccess {
    pub operator: bool,
    pub admin: bool,
    pub on_operator_page: bool,
}

impl NavAccess {
    pub async fn load(cx: &Cx, guild_id: Option<&str>) -> Self {
        let (operator, admin) = tokio::join!(is_operator(cx), is_admin(cx));
        let (operator, admin) = (operator.unwrap_or(false), admin.unwrap_or(false));
        let on_operator_page = match guild_id {
            Some(guild_id) if operator => {
                guild_operator_access(cx, guild_id).await.unwrap_or(false)
            },
            _ => false,
        };

        Self { operator, admin, on_operator_page }
    }
}

fn current(href: &str, location: &str, exact: bool) -> Option<&'static str> {
    let on = if exact { href == location } else { is_active_for(href, location) };
    on.then_some("page")
}

#[component]
pub async fn nav_links(
    cx: &Cx,
    prefix: &str,
    access: NavAccess,
    #[default] guild_id: Option<&str>,
) -> Result<impl View> {
    let location = uri(cx).path();
    let NavAccess { operator, admin, on_operator_page } = access;
    let list_location = guild_id.map_or_else(String::new, |guild_id| {
        module_list_location(guild_id, location)
    });
    let overview = guild_id.map(|guild_id| format!("/guild/{guild_id}"));
    let settings = guild_id.map(|guild_id| GENERAL.href(guild_id));

    Ok(view! {
        <nav class="nav" aria-label="Dashboard">
            if on_operator_page {
                <div class="operator-badge">
                    icon(name: Icon::Shield)
                    <span>"Operator access"</span>
                </div>
            }
            if let (Some(guild_id), Some(overview), Some(settings)) = (
                guild_id,
                overview.as_deref(),
                settings.as_deref(),
            ) {
                <ul class="nav-list">
                    nav_item(
                        href: overview,
                        label: OVERVIEW,
                        icon_name: Some(Icon::Grid),
                        aria_current: current(overview, location, true)
                    )
                    nav_item(
                        href: settings,
                        label: GENERAL.label,
                        icon_name: Some(Icon::Settings),
                        aria_current: current(settings, &list_location, false)
                    )
                </ul>
                #[key(group.label)]
                for group in GROUPS {
                    nav_group(
                        prefix: prefix,
                        label: group.label,
                        guild_id: guild_id,
                        entries: group.entries,
                        location: &list_location
                    )
                }
            } else {
                <ul class="nav-list">
                    nav_item(
                        href: "/guilds",
                        label: ALL_SERVERS,
                        icon_name: Some(Icon::Server),
                        aria_current: current("/guilds", location, true)
                    )
                </ul>
            }
            <ul class="nav-list nav-group">
                nav_item(
                    href: "/upgrade",
                    label: PLANS,
                    icon_name: Some(Icon::Zap),
                    aria_current: current("/upgrade", location, false)
                )
            </ul>
            if operator || admin {
                <div class="nav-group">
                    <p class="nav-heading" id=(format!("{prefix}-admin"))>"Admin"</p>
                    <ul class="nav-list" aria-labelledby=(format!("{prefix}-admin"))>
                        if operator {
                            nav_item(
                                href: "/admin/servers",
                                label: "All bot servers",
                                icon_name: Some(Icon::Shield),
                                aria_current: current("/admin/servers", location, true)
                            )
                        }
                        if admin {
                            nav_item(
                                href: "/admin/destiny2/loadouts",
                                label: "Loadout builder",
                                icon_name: Some(Icon::Gamepad),
                                aria_current: current(
                                    "/admin/destiny2/loadouts",
                                    location,
                                    false,
                                )
                            )
                        }
                    </ul>
                </div>
            }
        </nav>
    }
    .boxed())
}

#[component]
async fn nav_group(
    prefix: &str,
    label: &str,
    guild_id: &str,
    entries: &[ModuleNav],
    location: &str,
) -> Result<impl View> {
    let heading = format!(
        "{prefix}-{}",
        label
            .to_ascii_lowercase()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    );

    Ok(view! {
        <div class="nav-group">
            <p class="nav-heading" id=(heading.as_str())>(label)</p>
            <ul class="nav-list" aria-labelledby=(heading.as_str())>
                #[key(entry.label)]
                for entry in entries {
                    let href = entry.href(guild_id);
                    nav_item(
                        href: &href,
                        label: entry.label,
                        aria_current: current(&href, location, false)
                    )
                }
            </ul>
        </div>
    })
}

#[component]
async fn nav_item(
    href: &str,
    label: &str,
    #[default] icon_name: Option<Icon>,
    #[default] aria_current: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <li>
            <a href=(href) class="nav-link" aria-current=(aria_current)>
                if let Some(name) = icon_name {
                    icon(name: name)
                }
                <span>(label)</span>
            </a>
        </li>
    })
}
