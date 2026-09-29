use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use super::fields::{Datalist, OptionSelect, TextInput};
use super::state::{ArmourRow, AspectRow, EditorState, StatRow, WeaponRow};
use crate::dto::destiny2::{
    CatalogWeaponInfo,
    LoadoutCatalog,
    LoadoutForm,
    LoadoutOptions,
    WeaponForm,
};
use crate::server::destiny2::{SaveLoadout, get_loadout, loadout_catalog};
use crate::server::error::is_denied;
use crate::ui::components::key_list::KeyListField;
use crate::ui::components::layout::AppShell;
use crate::ui::components::settings::{SaveButton, save_feedback};
use crate::ui::components::skeleton::Skeleton;

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
                        Ok((catalog, form)) => view! { <Editor catalog form/> }.into_any(),
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}

fn choices(
    options: StoredValue<LoadoutOptions>,
    pick: fn(&LoadoutOptions) -> &Vec<String>,
) -> Vec<String> {
    options.with_value(|o| pick(o).clone())
}

#[component]
fn Editor(catalog: LoadoutCatalog, form: LoadoutForm) -> impl IntoView {
    let LoadoutCatalog { weapons, perks, emoji_keys, known_emoji, options, blank } =
        catalog;

    let state = EditorState::from_form(form);
    let save = ServerAction::<SaveLoadout>::new();
    let result = save.value();
    let navigate = use_navigate();
    let blank_weapon = blank.weapons.into_iter().next().unwrap_or_default();
    let weapon_names = weapons.iter().map(|w| w.name.clone()).collect::<Vec<_>>();
    let options = StoredValue::new(options);
    let weapons = StoredValue::new(weapons);
    let known = StoredValue::new(known_emoji);

    Effect::new(move |_| {
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

    let unknown = move || {
        known.with_value(|known| {
            if known.is_empty() {
                return Vec::new();
            }
            let mut missing = state.emoji_keys();
            missing.retain(|k| !known.contains(k));
            missing.sort();
            missing.dedup();
            missing
        })
    };

    view! {
        <Datalist id="emoji-keys" values=emoji_keys/>
        <Datalist id="perk-names" values=perks/>
        <Datalist id="weapon-names" values=weapon_names/>
        <form
            class="loadout-editor"
            on:submit=move |ev| {
                ev.prevent_default();
                save.dispatch(SaveLoadout { form: state.to_form() });
            }
        >
            <div class="page-header">
                <h1>{move || if state.id.get().is_some() { "Edit loadout" } else { "New loadout" }}</h1>
            </div>
            {move || result.get().map(|r| save_feedback(r.map(|_| ())))}
            {move || {
                let missing = unknown();
                (!missing.is_empty()).then(|| view! {
                    <p class="loadout-warning">
                        "Not among Zayden's emojis; /destiny2 builds cannot show this build until they are added: "
                        {missing.join(", ")}
                    </p>
                })
            }}
            <IdentitySection state options/>
            <AbilitiesSection state/>
            <AspectsSection state/>
            <WeaponsSection state options weapons blank_weapon/>
            <ArmourSection state/>
            <StatsSection state options/>
            <ArtifactSection state/>
            <SaveButton pending=save.pending()/>
        </form>
    }
}

#[component]
fn IdentitySection(
    state: EditorState,
    options: StoredValue<LoadoutOptions>,
) -> AnyView {
    view! {
        <fieldset class="settings-section loadout-grid">
            <legend>"Identity"</legend>
            <TextInput label="Name" value=state.name/>
            <OptionSelect label="Class" value=state.class options=choices(options, |o| &o.classes)/>
            <OptionSelect label="Subclass" value=state.element options=choices(options, |o| &o.elements)/>
            <OptionSelect label="Mode" value=state.mode options=choices(options, |o| &o.modes)/>
            <TextInput label="Author" value=state.author/>
            <TextInput label="DIM link" value=state.dim_link placeholder="https://dim.gg/…"/>
            <TextInput label="Video URL" value=state.video_url placeholder="https://youtu.be/…"/>
            <KeyListField label="Tags (max 3)" keys=state.tags list="" max=3/>
        </fieldset>
    }
    .into_any()
}

#[component]
fn AbilitiesSection(state: EditorState) -> AnyView {
    view! {
        <fieldset class="settings-section loadout-grid">
            <legend>"Abilities"</legend>
            <TextInput label="Super name" value=state.super_name/>
            <TextInput label="Super emoji" value=state.super_emoji list="emoji-keys"/>
            <TextInput label="Class ability" value=state.class_ability list="emoji-keys"/>
            <TextInput label="Jump" value=state.jump list="emoji-keys"/>
            <TextInput label="Melee" value=state.melee list="emoji-keys"/>
            <TextInput label="Grenade" value=state.grenade list="emoji-keys"/>
        </fieldset>
    }
    .into_any()
}

#[component]
fn AspectsSection(state: EditorState) -> AnyView {
    view! {
        <fieldset class="settings-section">
            <legend>"Aspects"</legend>
            <For
                each=move || state.aspects.get()
                key=|row| row.key
                children=move |row| view! { <AspectSlot state row/> }
            />
            <button
                type="button"
                class="btn btn-secondary"
                disabled=move || state.aspects.with(|a| a.len() >= 2)
                on:click=move |_| state.add_aspect()
            >
                "Add aspect"
            </button>
        </fieldset>
    }
    .into_any()
}

#[component]
fn AspectSlot(state: EditorState, row: AspectRow) -> AnyView {
    view! {
        <div class="loadout-slot">
            <TextInput label="Aspect" value=row.aspect list="emoji-keys"/>
            <KeyListField label="Fragments" keys=row.fragments list="emoji-keys" max=6/>
            <button
                type="button"
                class="btn btn-secondary"
                on:click=move |_| state.remove_aspect(row.key)
            >
                "Remove aspect"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn WeaponsSection(
    state: EditorState,
    options: StoredValue<LoadoutOptions>,
    weapons: StoredValue<Vec<CatalogWeaponInfo>>,
    blank_weapon: WeaponForm,
) -> AnyView {
    view! {
        <fieldset class="settings-section">
            <legend>"Weapons"</legend>
            <p class="loadout-hint">
                "Weapons, armour and tags must fit Discord's message size limit; \
                 a save that doesn't fit names what to trim."
            </p>
            <For
                each=move || state.weapons.get()
                key=|row| row.key
                children=move |row| view! { <WeaponSlot state options weapons row/> }
            />
            <button
                type="button"
                class="btn btn-secondary"
                disabled=move || state.weapons.with(|w| w.len() >= 3)
                on:click=move |_| state.add_weapon(blank_weapon.clone())
            >
                "Add weapon"
            </button>
        </fieldset>
    }
    .into_any()
}

#[component]
fn WeaponSlot(
    state: EditorState,
    options: StoredValue<LoadoutOptions>,
    weapons: StoredValue<Vec<CatalogWeaponInfo>>,
    row: WeaponRow,
) -> AnyView {
    let autofill = move || {
        weapons.with_value(|ws| {
            let name = row.name.get_untracked();
            if let Some(w) = ws.iter().find(|w| w.name == name) {
                row.affinity.set(w.affinity.clone());
                row.archetype.set(w.archetype.clone());
                row.icon_url.set(w.icon_url.clone());
            }
        });
    };

    view! {
        <div class="loadout-slot loadout-grid">
            <div class="setting-field">
                <label>"Name"</label>
                <input
                    class="input"
                    list="weapon-names"
                    prop:value=move || row.name.get()
                    on:input=move |ev| row.name.set(event_target_value(&ev))
                    on:change=move |_| autofill()
                />
            </div>
            <OptionSelect
                label="Affinity"
                value=row.affinity
                options=choices(options, |o| &o.affinities)
            />
            <OptionSelect
                label="Archetype"
                value=row.archetype
                options=choices(options, |o| &o.archetypes)
            />
            <TextInput label="Icon URL" value=row.icon_url placeholder="https://www.bungie.net/…"/>
            <KeyListField label="Perks" keys=row.perks list="perk-names" max=5/>
            <button
                type="button"
                class="btn btn-secondary"
                on:click=move |_| state.remove_weapon(row.key)
            >
                "Remove weapon"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn ArmourSection(state: EditorState) -> AnyView {
    view! {
        <fieldset class="settings-section">
            <legend>"Armour (leave a name blank to omit the slot)"</legend>
            <For
                each=move || state.armour.get()
                key=|row| row.key
                children=move |row| view! { <ArmourSlot row/> }
            />
        </fieldset>
    }
    .into_any()
}

#[component]
fn ArmourSlot(row: ArmourRow) -> AnyView {
    view! {
        <div class="loadout-slot loadout-grid">
            <p class="loadout-slot-title">{move || row.slot.get()}</p>
            <TextInput label="Name" value=row.name/>
            <TextInput label="Icon URL" value=row.icon_url placeholder="https://www.bungie.net/…"/>
            <KeyListField label="Mods" keys=row.mods list="emoji-keys" max=5/>
        </div>
    }
    .into_any()
}

#[component]
fn StatsSection(
    state: EditorState,
    options: StoredValue<LoadoutOptions>,
) -> AnyView {
    view! {
        <fieldset class="settings-section loadout-grid">
            <legend>"Stat priority (top first; blank value omits the row)"</legend>
            <For
                each=move || state.stats.get()
                key=|row| row.key
                children=move |row| view! { <StatSlot options row/> }
            />
        </fieldset>
    }
    .into_any()
}

#[component]
fn StatSlot(options: StoredValue<LoadoutOptions>, row: StatRow) -> AnyView {
    view! {
        <OptionSelect label="Stat" value=row.stat options=choices(options, |o| &o.stats)/>
        <TextInput label="Value" value=row.value placeholder="0-200"/>
    }
    .into_any()
}

#[component]
fn ArtifactSection(state: EditorState) -> AnyView {
    view! {
        <fieldset class="settings-section">
            <legend>"Artifact"</legend>
            <TextInput label="Artifact name" value=state.artifact_name/>
            <KeyListField label="Artifact perks" keys=state.artifact_perks list="emoji-keys" max=12/>
        </fieldset>
        <fieldset class="settings-section">
            <legend>"How it works"</legend>
            <textarea
                class="input"
                rows="8"
                prop:value=move || state.how_it_works.get()
                on:input=move |ev| state.how_it_works.set(event_target_value(&ev))
            ></textarea>
        </fieldset>
    }
    .into_any()
}
