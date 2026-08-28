use leptos::prelude::*;
use leptos_meta::Title;

use crate::server::guild::list_manageable_guilds;
use crate::ui::components::guild_grid::GuildGrid;
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
                        Ok(list) => view! { <GuildGrid guilds=list/> }.into_any(),
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
