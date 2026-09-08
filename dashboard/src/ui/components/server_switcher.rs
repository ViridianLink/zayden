use leptos::prelude::*;
use leptos_router::components::A;

use super::icons::Icon;
use crate::dto::GuildInfo;
use crate::server::guild::{get_active_guild, list_manageable_guilds};

fn guild_avatar(g: &GuildInfo) -> AnyView {
    g.icon.as_ref().map_or_else(
        || {
            let initial = g.name.chars().next().unwrap_or('#').to_string();
            view! {
                <span class="server-switcher-avatar placeholder">{initial}</span>
            }
            .into_any()
        },
        |hash| {
            let url = format!(
                "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                g.id, hash
            );
            view! { <img src=url alt="" class="server-switcher-avatar"/> }.into_any()
        },
    )
}

#[component]
pub(crate) fn ServerSwitcher(guild_id: Signal<String>) -> impl IntoView {
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());
    let active = Resource::new_blocking(move || guild_id.get(), get_active_guild);

    view! {
        <Suspense fallback=|| ()>
            {move || {
                active.get().and_then(Result::ok).map(|current| {
                    let list =
                        guilds.get().and_then(Result::ok).unwrap_or_default();
                    let avatar = guild_avatar(&current);
                    let current_id = current.id;
                    let current_name = current.name;

                    view! {
                        <details class="server-switcher">
                            <summary>
                                {avatar}
                                <span class="server-switcher-name">
                                    {current_name}
                                </span>
                                <Icon name="chevron-down"/>
                            </summary>
                            <div class="server-switcher-menu">
                                {list.into_iter().map(|g| {
                                    let is_current = g.id == current_id;
                                    let href = format!("/guild/{}", g.id);
                                    let opt_cls = if is_current {
                                        "server-switcher-option current"
                                    } else {
                                        "server-switcher-option"
                                    };
                                    let avatar = guild_avatar(&g);
                                    view! {
                                        <A href=href attr:class=opt_cls>
                                            {avatar}
                                            <span class="server-switcher-name">{g.name}</span>
                                            {is_current.then(|| view! { <Icon name="check"/> })}
                                        </A>
                                    }
                                }).collect_view()}
                            </div>
                        </details>
                    }
                })
            }}
        </Suspense>
    }
}
