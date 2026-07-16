use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::server::modules::{SetModuleEnabled, list_guild_modules};
use crate::ui::components::layout::AppShell;
use crate::ui::components::module_card::ModuleCard;

#[component]
pub(crate) fn GuildOverviewPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let toggle = ServerAction::<SetModuleEnabled>::new();
    let modules = Resource::new(
        move || (guild_id(), toggle.version().get()),
        |(gid, _)| list_guild_modules(gid),
    );

    let settings_href = move || format!("/guild/{}/settings", guild_id());

    view! {
        <Title text="Modules - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Modules"</h1>
                        <p class="page-lead">
                            "Turn modules on or off for this server. Toggling drives "
                            "Discord's own command-permission system."
                        </p>
                    </div>
                    <A href=settings_href attr:class="btn btn-secondary">"Server settings"</A>
                </div>
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
                                <div class="module-grid">
                                    {list.into_iter().map(|m| {
                                        view! {
                                            <ModuleCard
                                                module=m
                                                guild_id=gid.clone()
                                                toggle=toggle
                                            />
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
