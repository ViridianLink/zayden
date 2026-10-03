use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, component, view};

use super::link::aria_current;
use crate::components::guild_grid::GuildCard;
use crate::components::icons::{Icon, icon};
use crate::guild::dto::GuildInfo;
use crate::guild::get_active_guild;

/// A `<details>` menu of the guilds the user manages, headed by the guild
/// being viewed. Nothing renders when the viewed guild cannot be resolved.
#[component]
pub(super) async fn server_switcher(
    cx: &Cx,
    guild_id: &str,
    guilds: &[GuildInfo],
) -> Result<impl View> {
    let location = uri(cx).path();
    let current = get_active_guild(cx, guild_id)
        .await
        .ok()
        .map(|guild| GuildCard::from(&guild));

    Ok(view! {
        if let Some(current) = current {
            <details class="server-switcher">
                <summary>
                    switcher_avatar(guild: &current)
                    <span class="server-switcher-name">(current.name.as_str())</span>
                    icon(name: Icon::ChevronDown)
                </summary>
                <div class="server-switcher-menu">
                    #[key(guild.id.as_str())]
                    for guild in guilds {
                        let option = GuildCard::from(guild);
                        let href = option.href();
                        let is_current = option.id == current.id;
                        let class = if is_current {
                            "server-switcher-option current"
                        } else {
                            "server-switcher-option"
                        };
                        <a
                            href=(href.as_str())
                            aria-current=(aria_current(&href, location))
                            class=(class)
                        >
                            switcher_avatar(guild: &option)
                            <span class="server-switcher-name">
                                (option.name.as_str())
                            </span>
                            if is_current {
                                icon(name: Icon::Check)
                            }
                        </a>
                    }
                </div>
            </details>
        }
    })
}

#[component]
async fn switcher_avatar(guild: &GuildCard) -> Result<impl View> {
    Ok(view! {
        match guild.icon_url() {
            Some(url) => <img src=(url) alt="" class="server-switcher-avatar">,
            None => <span class="server-switcher-avatar placeholder">
                (guild.initial())
            </span>,
        }
    })
}
