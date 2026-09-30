use std::time::Duration;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use super::draft;
use super::extras::{ArtifactCard, DetailsCard, StatsCard};
use super::fields::TextInput;
use super::gear::GearCard;
use super::picker::{Picker, PickerCtx};
use super::ranking::CatalogIndex;
use super::slots::IconChoice;
use super::state::EditorState;
use super::subclass::SubclassCard;
use crate::dto::destiny2::{LoadoutCatalog, LoadoutForm};
use crate::server::destiny2::{
    CheckLoadout,
    SaveLoadout,
    get_loadout,
    loadout_catalog,
};
use crate::server::error::is_denied;
use crate::ui::components::layout::AppShell;
use crate::ui::components::settings::{SaveButton, save_feedback};
use crate::ui::components::skeleton::Skeleton;

const CHECK_DELAY: Duration = Duration::from_millis(400);

#[component]
pub(crate) fn LoadoutEditorPage() -> impl IntoView {
    let params = use_params_map();
    let id =
        move || params.with(|p| p.get("id").and_then(|id| id.parse::<i32>().ok()));

    let data = Resource::new_blocking(id, |id| async move {
        let catalog = loadout_catalog().await?;
        let form = match id {
            Some(id) => get_loadout(id).await?,
            None => catalog.blank.clone(),
        };
        Ok::<_, ServerFnError>((catalog, form))
    });

    view! {
        <Title text="Edit Loadout - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <Suspense fallback=|| view! {
                    <div class="skeleton-list"><Skeleton class="skeleton-row" count=8/></div>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) if is_denied(&e) => view! {
                            <p class="error">"Admin access is required to edit loadouts."</p>
                        }.into_any(),
                        Err(e) => view! {
                            <p class="error">"Couldn't load the loadout: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((catalog, form)) => view! { <ClientEditor catalog form/> }.into_any(),
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}

#[component]
fn ClientEditor(catalog: LoadoutCatalog, form: LoadoutForm) -> AnyView {
    let mounted = RwSignal::new(false);
    Effect::new(move |_| mounted.set(true));
    let data = StoredValue::new((catalog, form));

    view! {
        <Show
            when=move || mounted.get()
            fallback=|| view! { <div class="skeleton-list"><Skeleton class="skeleton-row" count=8/></div> }
        >
            {move || {
                let (catalog, form) = data.get_value();
                view! { <Editor catalog form/> }
            }}
        </Show>
    }
    .into_any()
}

#[component]
fn BudgetMeter(state: EditorState) -> AnyView {
    let check = ServerAction::<CheckLoadout>::new();
    let timer = StoredValue::new_local(None::<TimeoutHandle>);

    Effect::new(move |_| {
        let form = state.snapshot();
        if let Some(pending) = timer.get_value() {
            pending.clear();
        }
        let handle = set_timeout_with_handle(
            move || {
                check.dispatch(CheckLoadout { form });
            },
            CHECK_DELAY,
        );
        timer.set_value(handle.ok());
    });

    view! {
        <div class="budget" role="status" aria-live="polite">
            {move || check.value().get().map(|result| match result {
                Ok(c) => view! {
                    <div class="budget-meters">
                        <label class="budget-meter">
                            <span>{format!("{} / {} Discord components", c.components, c.max_components)}</span>
                            <meter min="0" max=c.max_components value=c.components high=c.max_components.saturating_sub(4)></meter>
                        </label>
                        {c.text.map(|text| view! {
                            <label class="budget-meter">
                                <span>{format!("~{text} / {} characters", c.max_text)}</span>
                                <meter min="0" max=c.max_text value=text high=c.max_text.saturating_sub(400)></meter>
                            </label>
                        })}
                    </div>
                    {c.error.map(|e| view! { <p class="budget-error">{e}</p> })}
                }.into_any(),
                Err(_) => view! {
                    <p class="loadout-hint">"Couldn't check Discord's limits right now."</p>
                }.into_any(),
            })}
        </div>
    }
    .into_any()
}

#[component]
fn Editor(catalog: LoadoutCatalog, form: LoadoutForm) -> AnyView {
    let key = StoredValue::new(draft::draft_key(form.id));
    let saved = StoredValue::new(form.clone());
    let state = EditorState::from_form(form);
    let ctx = PickerCtx::new(catalog, state.class, state.element);
    let restored = RwSignal::new(false);
    let checked_draft = StoredValue::new(false);

    Effect::new(move |_| {
        if !checked_draft.get_value() {
            checked_draft.set_value(true);
            if let Some(draft) = key.with_value(|k| draft::load(k))
                && saved.with_value(|s| *s != draft)
            {
                state.apply(draft);
                restored.set(true);
            }
        }
        let current = state.snapshot();
        key.with_value(|k| {
            if saved.with_value(|s| *s == current) {
                draft::clear(k);
            } else {
                draft::store(k, &current);
            }
        });
    });
    let discard = move |_| {
        key.with_value(|k| draft::clear(k));
        state.apply(saved.get_value());
        restored.set(false);
    };
    provide_context(ctx);
    let options = ctx.catalog.with_untracked(|c| c.options.clone());

    let save = ServerAction::<SaveLoadout>::new();
    let result = save.value();
    let navigate = use_navigate();

    Effect::new(move |_| {
        if matches!(result.get(), Some(Ok(_))) {
            key.with_value(|k| draft::clear(k));
            saved.set_value(state.to_form());
            restored.set(false);
        }
        if let Some(Ok(id)) = result.get()
            && state.id.get_untracked().is_none()
        {
            state.id.set(Some(id));
            navigate(
                &format!("/admin/destiny2/loadouts/{id}"),
                NavigateOptions::default(),
            );
        }
    });

    let missing = move || {
        ctx.index.with(|idx| {
            if !idx.has_any_emoji() {
                return Vec::new();
            }
            let mut missing = state.emoji_keys();
            missing.retain(|k| !idx.has_emoji(k));
            missing.sort();
            missing.dedup();
            missing
        })
    };
    let images_down = move || !ctx.index.with(CatalogIndex::has_any_emoji);

    view! {
        <form
            class="loadout-editor"
            on:submit=move |ev| {
                ev.prevent_default();
                save.dispatch(SaveLoadout { form: state.to_form() });
            }
        >
            <header class="loadout-header">
                <h1>{move || if state.id.get().is_some() { "Edit loadout" } else { "New loadout" }}</h1>
                <TextInput id="loadout-name" label="Build name" value=state.name/>
                <div class="loadout-choices">
                    <IconChoice label="Class" value=state.class options=options.classes/>
                    <IconChoice label="Subclass" value=state.element options=options.elements/>
                    <IconChoice label="Mode" value=state.mode options=options.modes/>
                </div>
                <BudgetMeter state/>
            </header>
            {move || result.get().map(|r| save_feedback(r.map(|_| ())))}
            <Show when=move || restored.get()>
                <div class="draft-banner" role="status">
                    <span>"Restored your unsaved changes from this tab."</span>
                    <button type="button" class="btn btn-ghost" on:click=discard>
                        "Discard changes"
                    </button>
                </div>
            </Show>
            <Show when=images_down>
                <p class="loadout-warning" role="status">
                    "Zayden's emoji list couldn't be loaded, so icons are hidden. You can still edit and save."
                </p>
            </Show>
            {move || {
                let missing = missing();
                (!missing.is_empty()).then(|| view! {
                    <p class="loadout-warning" role="status">
                        "Zayden has no emoji for these, so /destiny2 builds can't show this build until they're added: "
                        {missing.join(", ")}
                    </p>
                })
            }}
            <div class="loadout-board">
                <SubclassCard state/>
                <GearCard state/>
                <ArtifactCard state/>
                <StatsCard state/>
                <DetailsCard state/>
            </div>
            <SaveButton pending=save.pending()/>
        </form>
        <Picker/>
    }
    .into_any()
}
