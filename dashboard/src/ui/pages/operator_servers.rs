use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::server::operator::list_bot_guilds;
use crate::ui::components::guild_grid::GuildGrid;
use crate::ui::components::layout::AppShell;
use crate::ui::pages::not_found::NotFound;

#[must_use]
pub fn parse_guild_id(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();

    if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    trimmed.parse::<u64>().ok().filter(|id| *id != 0)
}

#[component]
pub(crate) fn OperatorServersPage() -> impl IntoView {
    let guilds = Resource::new_blocking(|| (), |()| list_bot_guilds());

    let filter = RwSignal::new(String::new());
    let jump = RwSignal::new(String::new());

    view! {
        <Title text="All Servers - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"All Servers"</h1>
                        <p class="page-lead">
                            "Every server Zayden is in. Operator access ignores \
                             your own permissions in them."
                        </p>
                    </div>
                </div>
                <Suspense fallback=|| view! { <p class="loading">"Loading servers\u{2026}"</p> }>
                    {move || guilds.get().map(|result| match result {
                        Err(_e) => view! { <NotFound/> }.into_any(),
                        Ok(list) => {
                            let total = list.len();

                            view! {
                                <div class="operator-tools">
                                    <input
                                        type="search"
                                        placeholder="Filter by name"
                                        aria-label="Filter servers by name"
                                        prop:value=move || filter.get()
                                        on:input=move |ev| filter.set(event_target_value(&ev))
                                    />
                                    <div class="operator-jump">
                                        <input
                                                type="text"
                                            inputmode="numeric"
                                            placeholder="Go to server ID"
                                            aria-label="Go to server ID"
                                            prop:value=move || jump.get()
                                            on:input=move |ev| jump.set(event_target_value(&ev))
                                        />
                                        {move || parse_guild_id(&jump.get()).map_or_else(
                                            || view! {
                                                <button
                                                    type="button"
                                                    class="btn btn-secondary"
                                                    disabled=true
                                                >
                                                    "Go"
                                                </button>
                                            }.into_any(),
                                            |id| view! {
                                                <A
                                                    href=format!("/guild/{id}")
                                                    attr:class="btn btn-secondary"
                                                >
                                                    "Go"
                                                </A>
                                            }.into_any(),
                                        )}
                                    </div>
                                </div>
                                {move || {
                                    let needle = filter.get().trim().to_lowercase();
                                    let shown: Vec<_> = if needle.is_empty() {
                                        list.clone()
                                    } else {
                                        list.iter()
                                            .filter(|g| {
                                                g.name.to_lowercase().contains(&needle)
                                            })
                                            .cloned()
                                            .collect()
                                    };

                                    if shown.is_empty() {
                                        return view! {
                                            <p class="empty">"No server matches that name."</p>
                                        }.into_any();
                                    }

                                    let count = shown.len();

                                    view! {
                                        <p class="operator-count">
                                            {count} " of " {total} " servers"
                                        </p>
                                        <GuildGrid guilds=shown/>
                                    }.into_any()
                                }}
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
