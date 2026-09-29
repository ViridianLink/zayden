use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;

use crate::dto::destiny2::LoadoutSummary;
use crate::server::destiny2::{DeleteLoadout, list_loadouts};
use crate::server::error::is_denied;
use crate::ui::components::confirm::ConfirmButton;
use crate::ui::components::layout::AppShell;
use crate::ui::components::settings::delete_feedback;
use crate::ui::components::skeleton::Skeleton;

#[component]
pub(crate) fn LoadoutListPage() -> impl IntoView {
    let delete = ServerAction::<DeleteLoadout>::new();
    let delete_result = delete.value();
    let loadouts =
        Resource::new(move || delete.version().get(), |_| list_loadouts());
    let class = RwSignal::new(String::new());

    view! {
        <Title text="Loadout Builder - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Destiny 2 Loadouts"</h1>
                        <p class="page-lead">
                            "Builds shown by /destiny2 builds. Saves apply to the bot immediately."
                        </p>
                    </div>
                    <A href="/admin/destiny2/loadouts/new" attr:class="btn btn-primary">
                        "New loadout"
                    </A>
                </div>
                {move || delete_result.get().map(delete_feedback)}
                <select
                    class="input loadout-filter"
                    aria-label="Filter by class"
                    on:change=move |ev| class.set(event_target_value(&ev))
                >
                    <option value="">"All classes"</option>
                    <option value="Hunter">"Hunter"</option>
                    <option value="Titan">"Titan"</option>
                    <option value="Warlock">"Warlock"</option>
                </select>
                <Transition fallback=|| view! {
                    <div class="skeleton-list"><Skeleton class="skeleton-row" count=6/></div>
                }>
                    {move || loadouts.get().map(|result| match result {
                        Err(e) if is_denied(&e) => view! {
                            <p class="error">"Admin access is required to edit loadouts."</p>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="error">"Couldn't load loadouts: " {e.to_string()}</p>
                        }.into_any(),
                        Ok(list) => view! {
                            <div class="loadout-table">
                                {list
                                    .into_iter()
                                    .filter(|l| class.with(|c| c.is_empty() || *c == l.class))
                                    .map(|l| view! { <LoadoutRow loadout=l delete=delete/> })
                                    .collect_view()}
                            </div>
                        }.into_any(),
                    })}
                </Transition>
            </div>
        </AppShell>
    }
}

#[component]
fn LoadoutRow(
    loadout: LoadoutSummary,
    delete: ServerAction<DeleteLoadout>,
) -> impl IntoView {
    let href = format!("/admin/destiny2/loadouts/{}", loadout.id);
    let meta = format!(
        "{} \u{2022} {} \u{2022} {} \u{2022} by {}",
        loadout.class, loadout.element, loadout.mode, loadout.author
    );

    view! {
        <div class="loadout-row">
            <A href=href attr:class="loadout-name">{loadout.name}</A>
            <span class="loadout-meta">{meta}</span>
            <ActionForm action=delete>
                <input type="hidden" name="id" value=loadout.id.to_string()/>
                <ConfirmButton
                    pending=delete.pending()
                    label="Delete"
                    prompt="This removes the build from /destiny2 builds for everyone. It cannot be undone."
                    confirm="Delete loadout"
                />
            </ActionForm>
        </div>
    }
}
