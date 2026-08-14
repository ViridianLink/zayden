use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::server::levels::get_leaderboard;
use crate::ui::components::layout::AppShell;

const PAGE_SIZE: usize = 10;

#[component]
pub(crate) fn LevelsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let (global, set_global) = signal(false);
    let (page, set_page) = signal(1i32);

    let board = Resource::new(
        move || (guild_id(), global.get(), page.get()),
        |(gid, g, p)| get_leaderboard(gid, g, p),
    );

    view! {
        <Title text="Levels - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Levels"</h1>
                        <p class="page-lead">
                            "Message-XP rankings. Switch between this server and the global board."
                        </p>
                    </div>
                    <div class="segmented" role="tablist">
                        <button
                            type="button"
                            class=move || if global.get() { "seg" } else { "seg active" }
                            on:click=move |_| { set_global.set(false); set_page.set(1); }
                        >"This server"</button>
                        <button
                            type="button"
                            class=move || if global.get() { "seg active" } else { "seg" }
                            on:click=move |_| { set_global.set(true); set_page.set(1); }
                        >"Global"</button>
                    </div>
                </div>

                <Suspense fallback=|| view! {
                    <p class="loading">"Loading leaderboard\u{2026}"</p>
                }>
                    {move || board.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load leaderboard: " {e.to_string()}</p>
                        }.into_any(),
                        Ok(entries) if entries.is_empty() => view! {
                            <div class="empty">
                                {if page.get() > 1 {
                                    "No more entries on this page."
                                } else if global.get() {
                                    "No one has earned global XP yet."
                                } else {
                                    "No one has chatted here yet - the board fills as members talk."
                                }}
                            </div>
                        }.into_any(),
                        Ok(entries) => {
                            let has_next = entries.len() == PAGE_SIZE;
                            view! {
                                <div class="leaderboard">
                                    <div class="lb-row lb-head">
                                        <span class="lb-rank">"#"</span>
                                        <span class="lb-user">"Member"</span>
                                        <span class="lb-num">"Level"</span>
                                        <span class="lb-num">"XP"</span>
                                        <span class="lb-num">"Messages"</span>
                                    </div>
                                    {entries.into_iter().map(|e| view! {
                                        <div class="lb-row">
                                            <span class="lb-rank">{e.rank}</span>
                                            <span class="lb-user">
                                                {e.avatar.map_or_else(
                                                    || view! {
                                                        <span class="lb-avatar placeholder"></span>
                                                    }.into_any(),
                                                    |url| view! {
                                                        <img class="lb-avatar" src=url alt=""/>
                                                    }.into_any(),
                                                )}
                                                <span class="lb-name">{e.name}</span>
                                            </span>
                                            <span class="lb-num">{e.level}</span>
                                            <span class="lb-num">{e.xp}</span>
                                            <span class="lb-num">{e.message_count}</span>
                                        </div>
                                    }).collect_view()}
                                </div>
                                <div class="pager">
                                    <button
                                        type="button"
                                        class="btn btn-secondary"
                                        prop:disabled=move || page.get() <= 1
                                        on:click=move |_| set_page.update(|p| *p = (*p - 1).max(1))
                                    >"Previous"</button>
                                    <span class="pager-page">"Page " {move || page.get()}</span>
                                    <button
                                        type="button"
                                        class="btn btn-secondary"
                                        prop:disabled=!has_next
                                        on:click=move |_| set_page.update(|p| *p += 1)
                                    >"Next"</button>
                                </div>
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
