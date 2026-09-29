use leptos::prelude::*;

use super::picker::PickerCtx;
use super::ranking::Field;
use super::slots::{KeySlot, KeySlotList};
use super::state::{AspectRow, EditorState};
use crate::dto::destiny2_keys::display_name;
use crate::ui::components::icons::Icon;

const MAX_ASPECTS: usize = 2;

#[component]
pub fn SubclassCard(state: EditorState) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let name_super = Callback::new(move |key: String| {
        let known = ctx.catalog.with_untracked(|c| {
            c.super_names
                .iter()
                .find(|(emoji, _)| *emoji == key)
                .map(|(_, name)| name.clone())
        });
        state.super_name.set(known.unwrap_or_else(|| display_name(&key)));
    });

    view! {
        <section class="settings-section loadout-card" aria-labelledby="subclass-heading">
            <h2 id="subclass-heading" class="loadout-card-title">"Subclass"</h2>
            <div class="subclass-top">
                <div class="super-field">
                    <KeySlot label="Super" value=state.super_emoji field=Field::Super diamond=true on_picked=name_super/>
                </div>
                <div class="ability-row" role="group" aria-label="Abilities">
                    <KeySlot label="Class ability" value=state.class_ability field=Field::ClassAbility/>
                    <KeySlot label="Jump" value=state.jump field=Field::Jump/>
                    <KeySlot label="Melee" value=state.melee field=Field::Melee/>
                    <KeySlot label="Grenade" value=state.grenade field=Field::Grenade/>
                </div>
            </div>
            <div class="aspect-list">
                <For
                    each=move || state.aspects.get()
                    key=|row| row.key
                    children=move |row| view! { <AspectBlock state row/> }
                />
                <Show when=move || state.aspects.with(|a| a.len() < MAX_ASPECTS)>
                    <button type="button" class="btn btn-secondary" on:click=move |_| state.add_aspect()>
                        <Icon name="plus"/>
                        "Add aspect"
                    </button>
                </Show>
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn AspectBlock(state: EditorState, row: AspectRow) -> AnyView {
    view! {
        <div class="aspect-block">
            <KeySlot label="Aspect" value=row.aspect field=Field::Aspect/>
            <div class="aspect-fragments">
                <span class="label">"Fragments"</span>
                <KeySlotList label="Fragments" values=row.fragments field=Field::Fragment max=6/>
            </div>
            <button
                type="button"
                class="btn btn-ghost aspect-remove"
                on:click=move |_| state.remove_aspect(row.key)
            >
                "Remove aspect"
            </button>
        </div>
    }
    .into_any()
}
