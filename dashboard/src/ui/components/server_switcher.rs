use leptos::prelude::*;
use leptos_router::components::A;

use super::icons::Icon;
use crate::dto::GuildInfo;
use crate::server::guild::list_manageable_guilds;

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
pub(crate) fn ServerSwitcher(guild_id: String) -> impl IntoView {
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());

    view! {
        <Suspense fallback=|| ()>
            {move || {
                let current = guild_id.clone();
                guilds.get().and_then(Result::ok).map(|list| {
                    let active_name = list
                        .iter()
                        .find(|g| g.id == current)
                        .map_or_else(
                            || "Select a server".to_string(),
                            |g| g.name.clone(),
                        );
                    let active_avatar = list
                        .iter()
                        .find(|g| g.id == current)
                        .map(guild_avatar);

                    view! {
                        <details class="server-switcher">
                            <summary>
                                {active_avatar}
                                <span class="server-switcher-name">{active_name}</span>
                                <Icon name="chevron-down"/>
                            </summary>
                            <div class="server-switcher-menu">
                                {list.into_iter().map(|g| {
                                    let is_current = g.id == current;
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
