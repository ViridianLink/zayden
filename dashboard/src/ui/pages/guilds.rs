use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::server::guild::list_manageable_guilds;

#[component]
pub(crate) fn GuildListPage() -> impl IntoView {
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());

    view! {
        <Title text="Servers - Zayden Dashboard"/>
        <div class="page">
            <h1>"Your Servers"</h1>
            <Suspense fallback=|| view! { <p class="loading">"Loading servers\u{2026}"</p> }>
                {move || guilds.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load servers: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(list) if list.is_empty() => view! {
                        <p class="empty">"You manage no servers with this account."</p>
                    }.into_any(),
                    Ok(list) => view! {
                        <ul class="guild-list">
                            {list.into_iter().map(|g| {
                                let icon_url = g.icon.map(|hash| {
                                    format!(
                                        "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                                        g.id, hash,
                                    )
                                });
                                let href = format!("/guild/{}/settings", g.id);
                                view! {
                                    <li class="guild-item">
                                        <A href=href>
                                            {icon_url.map(|url| view! {
                                                <img src=url alt="" class="guild-icon"/>
                                            })}
                                            <span class="guild-name">{g.name}</span>
                                        </A>
                                    </li>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_any(),
                })}
            </Suspense>
        </div>
    }
}
