#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, ViewExt, component, view};

use super::guild_grid::GuildCard;
use super::icons::{Icon, icon};
use crate::guild::dto::GuildInfo;
use crate::shell::link::switch_href;

pub const FILTER_THRESHOLD: usize = 8;
pub const PANEL_ID: &str = "server-switcher";

#[component]
pub async fn server_plate(
    cx: &Cx,
    current: Option<&GuildCard>,
    guilds: &[GuildInfo],
) -> Result<impl View> {
    let location = uri(cx).path();
    let filterable = guilds.len() > FILTER_THRESHOLD;

    Ok(view! {
        if let Some(current) = current {
            <button
                type="button"
                class="topbar-button plate"
                popovertarget=(PANEL_ID)
                aria-controls=(PANEL_ID)
                aria-expanded="false"
                aria-label=(format!("Switch server, current: {}", current.name))
            >
                plate_avatar(guild: current)
                <span class="plate-name">(current.name.as_str())</span>
                icon(name: Icon::ChevronDown)
            </button>
            <div
                id=(PANEL_ID)
                popover=""
                class="popover-panel menu-panel menu-panel-start"
                data-filter=""
                data-filter-noun="servers"
            >
                if filterable {
                    <div class="menu-filter">
                        <label class="visually-hidden" for="server-filter">
                            "Filter servers"
                        </label>
                        <input
                            id="server-filter"
                            type="search"
                            class="input"
                            placeholder="Filter servers"
                            autocomplete="off"
                            data-filter-input=""
                            autofocus=""
                        >
                    </div>
                    <p class="menu-count" role="status" data-filter-count=""></p>
                }
                <div class="menu-list" data-filter-list="">
                    #[key(guild.id.as_str())]
                    for guild in guilds {
                        let option = GuildCard::from(guild);
                        let is_current = option.id == current.id;
                        let href = switch_href(&current.id, location, &option.id);
                        <a
                            href=(href.as_str())
                            class="menu-item"
                            aria-current=(is_current.then_some("page"))
                            data-filter-item=""
                        >
                            plate_avatar(guild: &option)
                            <span class="plate-name">(option.name.as_str())</span>
                            if is_current {
                                icon(name: Icon::Check)
                            }
                        </a>
                    }
                </div>
                <a href="/guilds" class="menu-item">
                    icon(name: Icon::Server)
                    <span class="plate-name">"All servers"</span>
                </a>
            </div>
        }
    }
    .boxed())
}

#[component]
async fn plate_avatar(guild: &GuildCard) -> Result<impl View> {
    Ok(view! {
        match guild.icon_url() {
            Some(url) => <img
                src=(url)
                alt=""
                width="24"
                height="24"
                class="plate-avatar"
            >,
            None => <span class="plate-avatar placeholder" aria-hidden="true">
                (guild.initial())
            </span>,
        }
    })
}
