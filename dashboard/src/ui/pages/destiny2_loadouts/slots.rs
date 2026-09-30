use leptos::prelude::*;

use super::picker::{PickRequest, Picked, PickerCtx};
use super::ranking::Field;
use super::reorder::{Axis, Reorder};
use super::state::move_item;
use crate::dto::destiny2_keys::display_name;
use crate::ui::components::icons::Icon;

#[component]
pub fn KeyImage(key: String) -> impl IntoView {
    let ctx = expect_context::<PickerCtx>();
    let title = key.clone();
    move || {
        ctx.image(&key).map_or_else(
            || view! {
                <span class="slot-missing" title=format!("Zayden has no emoji named {title}")>"?"</span>
            }
            .into_any(),
            |src| view! { <img src=src alt="" loading="lazy"/> }.into_any(),
        )
    }
}

#[component]
pub fn EnumIcon(#[prop(into)] label: Signal<String>) -> impl IntoView {
    let ctx = expect_context::<PickerCtx>();
    move || {
        ctx.index
            .with(|idx| idx.enum_image(&label.get()))
            .map(|src| view! { <img class="enum-icon" src=src alt=""/> })
    }
}

fn open_key_picker(
    ctx: PickerCtx,
    field: Field,
    title: String,
    weapon: Option<String>,
    taken: Vec<String>,
    on_key: impl Fn(String) + Send + Sync + 'static,
) {
    ctx.open(PickRequest {
        field,
        title,
        weapon,
        armour_slot: None,
        taken,
        on_pick: Callback::new(move |picked| {
            if let Picked::Key(key) = picked {
                on_key(key);
            }
        }),
    });
}

#[component]
fn SlotFace(
    #[prop(into)] key: Signal<String>,
    label: String,
    diamond: bool,
    on_open: Callback<()>,
    on_clear: Callback<()>,
    #[prop(optional)] id: Option<String>,
) -> AnyView {
    let filled = move || key.with(|k| !k.is_empty());
    let aria = {
        let label = label.clone();
        move || {
            let k = key.get();
            if k.is_empty() {
                format!("{label}: empty, choose one")
            } else {
                format!("{label}: {}, change", display_name(&k))
            }
        }
    };
    let tooltip = move || {
        let k = key.get();
        (!k.is_empty()).then(|| format!("{} ({k})", display_name(&k)))
    };

    view! {
        <div class="slot" class:slot-diamond=diamond class:slot-filled=filled>
            <button
                type="button"
                class="slot-button"
                id=id
                aria-label=aria
                title=tooltip
                on:click=move |_| on_open.run(())
            >
                {move || {
                    let k = key.get();
                    if k.is_empty() {
                        view! { <span class="slot-plus" aria-hidden="true"><Icon name="plus"/></span> }.into_any()
                    } else {
                        view! { <KeyImage key=k/> }.into_any()
                    }
                }}
            </button>
            <Show when=filled>
                <button
                    type="button"
                    class="slot-clear"
                    aria-label=format!("Remove {label}")
                    on:click=move |_| on_clear.run(())
                >
                    <Icon name="x"/>
                </button>
            </Show>
        </div>
    }
    .into_any()
}

#[component]
pub fn KeySlot(
    label: &'static str,
    value: RwSignal<String>,
    field: Field,
    #[prop(optional)] diamond: bool,
    #[prop(optional)] on_picked: Option<Callback<String>>,
) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let on_open = Callback::new(move |()| {
        open_key_picker(
            ctx,
            field,
            label.to_owned(),
            None,
            Vec::new(),
            move |key| {
                value.set(key.clone());
                if let Some(cb) = on_picked {
                    cb.run(key);
                }
            },
        );
    });

    view! {
        <div class="slot-cell">
            <SlotFace
                key=value
                label=label.to_owned()
                diamond
                on_open
                on_clear=Callback::new(move |()| value.set(String::new()))
            />
            <span class="slot-caption">{label}</span>
        </div>
    }
    .into_any()
}

#[component]
pub fn KeySlotList(
    label: &'static str,
    values: RwSignal<Vec<String>>,
    field: Field,
    max: usize,
    #[prop(optional, into)] weapon: Option<Signal<String>>,
    #[prop(optional)] round: bool,
    #[prop(optional)] reorderable: bool,
) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let reorder =
        Reorder::new(move |from, to| values.update(|v| move_item(v, from, to)));
    let weapon_name =
        move || weapon.map(|w| w.get_untracked()).filter(|w| !w.is_empty());
    let noun = field.noun();

    let slot = move |index: usize, key: String| {
        let open = Callback::new(move |()| {
            let taken = if field.allows_repeats() {
                Vec::new()
            } else {
                values.get_untracked()
            };
            let title = format!("{label}: {noun} {}", index + 1);
            open_key_picker(ctx, field, title, weapon_name(), taken, move |key| {
                values.update(|v| match v.get_mut(index) {
                    Some(slot) => *slot = key,
                    None => v.push(key),
                });
            });
        });
        let clear = Callback::new(move |()| {
            values.update(|v| {
                if index < v.len() {
                    v.remove(index);
                }
            });
        });
        let filled = !key.is_empty();
        let face = view! {
            <SlotFace
                key=key
                label=format!("{label} {noun} {}", index + 1)
                diamond=false
                on_open=open
                on_clear=clear
                id=reorder.item_id(index)
            />
        };
        if !(reorderable && filled) {
            return face.into_any();
        }
        view! {
            <div
                class="slot-drag"
                draggable="true"
                title="Drag, or press Alt + arrow keys, to reorder"
                class:drop-target=move || reorder.is_target(index)
                on:dragstart=move |ev| reorder.start(index, &ev)
                on:dragover=move |ev| reorder.over(index, &ev)
                on:drop=move |ev| reorder.drop(index, &ev)
                on:dragend=move |_| reorder.end()
                on:keydown=move |ev| {
                    reorder.key(index, values.with_untracked(Vec::len), Axis::Row, true, &ev);
                }
            >
                {face}
            </div>
        }
        .into_any()
    };

    view! {
        <div class="slot-list" class:slot-list-round=round role="group" aria-label=label>
            <For
                each=move || {
                    let keys = values.get();
                    let len = keys.len();
                    keys.into_iter()
                        .enumerate()
                        .chain((len < max).then(|| (len, String::new())))
                        .collect::<Vec<_>>()
                }
                key=|slot| slot.clone()
                children=move |(i, k)| slot(i, k)
            />
            <span class="slot-count" aria-hidden="true">
                {move || format!("{}/{max}", values.with(Vec::len))}
            </span>
        </div>
    }
    .into_any()
}

#[component]
pub fn IconChoice(
    label: &'static str,
    value: RwSignal<String>,
    options: Vec<String>,
) -> AnyView {
    view! {
        <div class="icon-choice-field" role="radiogroup" aria-label=label>
            <span class="label">{label}</span>
            <div class="icon-choice">
                {options.into_iter().map(|option| {
                    let checked = {
                        let option = option.clone();
                        Signal::derive(move || value.with(|v| *v == option))
                    };
                    let pick = option.clone();
                    let text = option.clone();
                    view! {
                        <button
                            type="button"
                            role="radio"
                            class="icon-choice-option"
                            aria-checked=move || checked.get().to_string()
                            class:active=checked
                            on:click=move |_| value.set(pick.clone())
                        >
                            <EnumIcon label=option/>
                            <span>{text}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}
