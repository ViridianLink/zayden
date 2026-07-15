use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::server::modules::{SetModuleEnabled, list_guild_modules};

#[component]
pub(crate) fn ModulesPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let toggle = ServerAction::<SetModuleEnabled>::new();
    let modules = Resource::new(
        move || (guild_id(), toggle.version().get()),
        |(gid, _)| list_guild_modules(gid),
    );

    view! {
        <Title text="Modules - Zayden Dashboard"/>
        <div class="page">
            <h1>"Modules"</h1>
            <p class="page-lead">
                "Turn modules on or off for this server. Toggling drives Discord's "
                "own command-permission system, so Discord enforces and persists it."
            </p>
            {move || toggle.value().get().and_then(Result::err).map(|e| view! {
                <p class="error">{e.to_string()}</p>
            })}
            <Suspense fallback=|| view! { <p class="loading">"Loading modules\u{2026}"</p> }>
                {move || modules.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load modules: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(list) => {
                        let gid = guild_id();
                        view! {
                            <ul class="module-list">
                                {list.into_iter().map(|m| {
                                    let gid = gid.clone();
                                    let module_id = m.id.clone();
                                    let enabled = m.enabled;
                                    let toggle_cls = if enabled {
                                        "toggle toggle-on"
                                    } else {
                                        "toggle"
                                    };
                                    let commands = m.commands.join(", ");
                                    view! {
                                        <li class="module-row">
                                            <div class="module-meta">
                                                <span class="module-name">{m.label}</span>
                                                <span>{m.description}</span>
                                                <span class="module-commands">{commands}</span>
                                            </div>
                                            <button
                                                class=toggle_cls
                                                aria-label="Toggle module"
                                                on:click=move |_| {
                                                    toggle.dispatch(SetModuleEnabled {
                                                        guild: gid.clone(),
                                                        module_id: module_id.clone(),
                                                        enabled: !enabled,
                                                    });
                                                }
                                            />
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        }.into_any()
                    },
                })}
            </Suspense>
        </div>
    }
}
