use leptos::prelude::*;

use super::picker::{PickRequest, Picked, PickerCtx};
use super::ranking::Field;
use super::slots::{EnumIcon, KeySlotList};
use super::state::{ArmourRow, EditorState, WeaponRow};
use crate::dto::destiny2::WeaponForm;
use crate::ui::components::icons::Icon;

const MAX_WEAPONS: usize = 3;

#[component]
fn ItemSlot(
    label: String,
    #[prop(into)] name: Signal<String>,
    #[prop(into)] icon: Signal<String>,
    on_open: Callback<()>,
    #[prop(optional)] on_clear: Option<Callback<()>>,
) -> AnyView {
    let filled = move || name.with(|n| !n.is_empty());
    let aria = {
        let label = label.clone();
        move || {
            let n = name.get();
            if n.is_empty() {
                format!("{label}: empty, choose one")
            } else {
                format!("{label}: {n}, change")
            }
        }
    };

    view! {
        <div class="slot slot-item" class:slot-filled=filled>
            <button type="button" class="slot-button" aria-label=aria title=move || name.get() on:click=move |_| on_open.run(())>
                {move || {
                    let src = icon.get();
                    if !filled() {
                        view! { <span class="slot-plus" aria-hidden="true"><Icon name="plus"/></span> }.into_any()
                    } else if src.is_empty() {
                        view! { <span class="slot-missing" aria-hidden="true">"?"</span> }.into_any()
                    } else {
                        view! { <img src=src alt="" loading="lazy"/> }.into_any()
                    }
                }}
            </button>
            {on_clear.map(|clear| view! {
                <Show when=filled>
                    <button type="button" class="slot-clear" aria-label=format!("Remove {label}") on:click=move |_| clear.run(())>
                        <Icon name="x"/>
                    </button>
                </Show>
            })}
        </div>
    }
    .into_any()
}

fn open_weapon_picker(
    ctx: PickerCtx,
    title: String,
    on_weapon: impl Fn(WeaponForm) + Send + Sync + 'static,
) {
    ctx.open(PickRequest {
        field: Field::Weapon,
        title,
        weapon: None,
        armour_slot: None,
        taken: Vec::new(),
        on_pick: Callback::new(move |picked| {
            if let Picked::Weapon(w) = picked {
                on_weapon(WeaponForm {
                    name: w.name,
                    affinity: w.affinity,
                    archetype: w.archetype,
                    icon_url: w.icon_url,
                    perks: Vec::new(),
                });
            }
        }),
    });
}

#[component]
pub fn GearCard(state: EditorState) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let add_weapon = Callback::new(move |()| {
        open_weapon_picker(ctx, "Add weapon".to_owned(), move |w| {
            state.add_weapon(w);
        });
    });

    view! {
        <section class="settings-section loadout-card" aria-labelledby="gear-heading">
            <h2 id="gear-heading" class="loadout-card-title">"Gear and mods"</h2>
            <h3 class="loadout-subtitle">"Weapons"</h3>
            <div class="gear-list">
                <For
                    each=move || state.weapons.get()
                    key=|row| row.key
                    children=move |row| view! { <WeaponLine state row/> }
                />
                <Show when=move || state.weapons.with(|w| w.len() < MAX_WEAPONS)>
                    <div class="gear-row">
                        <ItemSlot label="New weapon".to_owned() name=String::new() icon=String::new() on_open=add_weapon/>
                        <div class="gear-info">
                            <span class="gear-name gear-empty">"Add a weapon"</span>
                            <span class="gear-meta">{move || format!("{} of {MAX_WEAPONS}", state.weapons.with(Vec::len))}</span>
                        </div>
                    </div>
                </Show>
            </div>
            <h3 class="loadout-subtitle">"Armour"</h3>
            <p class="loadout-hint">"Leave a slot empty to leave it out of the build."</p>
            <div class="gear-list">
                <For
                    each=move || state.armour.get()
                    key=|row| row.key
                    children=move |row| view! { <ArmourLine row/> }
                />
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn WeaponLine(state: EditorState, row: WeaponRow) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let replace = Callback::new(move |()| {
        open_weapon_picker(ctx, "Weapon".to_owned(), move |w| {
            if row.name.get_untracked() != w.name {
                row.perks.set(Vec::new());
            }
            row.name.set(w.name);
            row.affinity.set(w.affinity);
            row.archetype.set(w.archetype);
            row.icon_url.set(w.icon_url);
        });
    });

    view! {
        <div class="gear-row">
            <ItemSlot label="Weapon".to_owned() name=row.name icon=row.icon_url on_open=replace/>
            <div class="gear-info">
                <span class="gear-name">{move || row.name.get()}</span>
                <span class="gear-meta">
                    <EnumIcon label=row.affinity/>
                    {move || format!("{} {}", row.affinity.get(), row.archetype.get())}
                </span>
            </div>
            <KeySlotList label="Perks" values=row.perks field=Field::WeaponPerk max=5 weapon=row.name round=true/>
            <button
                type="button"
                class="btn btn-ghost gear-remove"
                aria-label=move || format!("Remove {}", row.name.get())
                on:click=move |_| state.remove_weapon(row.key)
            >
                <Icon name="x"/>
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn ArmourLine(row: ArmourRow) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let slot = row.slot.get_untracked();
    let open = {
        let slot = slot.clone();
        Callback::new(move |()| {
            ctx.open(PickRequest {
                field: Field::Armour,
                title: slot.clone(),
                weapon: None,
                armour_slot: Some(slot.clone()),
                taken: Vec::new(),
                on_pick: Callback::new(move |picked| {
                    if let Picked::Armour { name, icon_url } = picked {
                        row.name.set(name);
                        row.icon_url.set(icon_url);
                    }
                }),
            });
        })
    };
    let clear = Callback::new(move |()| {
        row.name.set(String::new());
        row.icon_url.set(String::new());
    });

    view! {
        <div class="gear-row">
            <ItemSlot label=slot.clone() name=row.name icon=row.icon_url on_open=open on_clear=clear/>
            <div class="gear-info">
                <span class="gear-name" class:gear-empty=move || row.name.with(String::is_empty)>
                    {move || { let n = row.name.get(); if n.is_empty() { "Empty".to_owned() } else { n } }}
                </span>
                <span class="gear-meta"><EnumIcon label=slot.clone()/>{slot}</span>
            </div>
            <KeySlotList label="Mods" values=row.mods field=Field::ArmourMod max=5 reorderable=true/>
        </div>
    }
    .into_any()
}
