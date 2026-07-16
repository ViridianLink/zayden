use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::server::guild::list_manageable_guilds;
use crate::ui::components::layout::AppShell;

#[component]
pub(crate) fn GuildListPage() -> impl IntoView {
    let guilds = Resource::new_blocking(|| (), |()| list_manageable_guilds());

    view! {
        <Title text="Servers - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Your Servers"</h1>
                        <p class="page-lead">"Pick a server to configure Zayden."</p>
                    </div>
                    <a href="/invite" rel="external" class="btn btn-secondary">"Add to a server"</a>
                </div>
                <Suspense fallback=|| view! { <p class="loading">"Loading servers\u{2026}"</p> }>
                    {move || guilds.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load servers: " {e.to_string()}</p>
                        }.into_any(),
                        Ok(list) if list.is_empty() => view! {
                            <p class="empty">"You manage no servers with this account."</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <div class="guild-grid">
                                {list.into_iter().map(|g| {
                                    let icon_url = g.icon.map(|hash| {
                                        format!(
                                            "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                                            g.id, hash,
                                        )
                                    });
                                    let initial = g.name.chars().next()
                                        .unwrap_or('#').to_string();
                                    let href = format!("/guild/{}", g.id);
                                    view! {
                                        <A href=href attr:class="guild-card">
                                            {icon_url.map_or_else(
                                                || view! {
                                                    <span class="guild-icon placeholder">
                                                        {initial}
                                                    </span>
                                                }.into_any(),
                                                |url| view! {
                                                    <img src=url alt="" class="guild-icon"/>
                                                }.into_any(),
                                            )}
                                            <div class="guild-card-body">
                                                <div class="guild-name">{g.name}</div>
                                                <div class="guild-card-hint">"Manage \u{2192}"</div>
                                            </div>
                                        </A>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any(),
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
