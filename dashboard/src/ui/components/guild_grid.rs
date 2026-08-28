use leptos::prelude::*;
use leptos_router::components::A;

use crate::dto::GuildInfo;

#[component]
pub(crate) fn GuildGrid(guilds: Vec<GuildInfo>) -> impl IntoView {
    view! {
        <div class="guild-grid">
            {guilds.into_iter().map(|g| {
                let icon_url = g.icon.map(|hash| {
                    format!(
                        "https://cdn.discordapp.com/icons/{}/{}.png?size=64",
                        g.id, hash,
                    )
                });
                let initial = g.name.chars().next().unwrap_or('#').to_string();
                let href = format!("/guild/{}", g.id);

                view! {
                    <A href=href attr:class="guild-card">
                        {icon_url.map_or_else(
                            || view! {
                                <span class="guild-icon placeholder">{initial}</span>
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
    }
}
