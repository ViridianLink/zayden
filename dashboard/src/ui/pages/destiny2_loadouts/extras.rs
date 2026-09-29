use leptos::prelude::*;

use super::fields::TextInput;
use super::picker::{PickRequest, Picked, PickerCtx};
use super::ranking::Field;
use super::reorder::{Axis, Reorder};
use super::slots::{EnumIcon, KeySlotList};
use super::state::{EditorState, StatRow};
use crate::ui::components::icons::Icon;
use crate::ui::components::key_list::KeyListField;

#[component]
pub fn ArtifactCard(state: EditorState) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let open = move |_| {
        ctx.open(PickRequest {
            field: Field::Artifact,
            title: "Artifact".to_owned(),
            weapon: None,
            armour_slot: None,
            taken: Vec::new(),
            on_pick: Callback::new(move |picked| {
                if let Picked::Key(name) = picked {
                    state.artifact_name.set(name);
                }
            }),
        });
    };
    let named = move || state.artifact_name.with(|n| !n.is_empty());

    view! {
        <section class="settings-section loadout-card" aria-labelledby="artifact-heading">
            <h2 id="artifact-heading" class="loadout-card-title">"Artifact"</h2>
            <div class="artifact-row">
                <button
                    type="button"
                    class="artifact-choice"
                    class:artifact-empty=move || !named()
                    aria-label=move || {
                        let n = state.artifact_name.get();
                        if n.is_empty() { "Artifact: none, choose one".to_owned() } else { format!("Artifact: {n}, change") }
                    }
                    on:click=open
                >
                    {move || {
                        let n = state.artifact_name.get();
                        if n.is_empty() { "Choose artifact".to_owned() } else { n }
                    }}
                </button>
                <Show when=named>
                    <button
                        type="button"
                        class="btn btn-ghost"
                        on:click=move |_| state.artifact_name.set(String::new())
                    >
                        "Clear"
                    </button>
                </Show>
            </div>
            <span class="label">"Artifact perks"</span>
            <KeySlotList
                label="Artifact perks"
                values=state.artifact_perks
                field=Field::ArtifactPerk
                max=12
                reorderable=true
            />
        </section>
    }
    .into_any()
}

#[component]
pub fn StatsCard(state: EditorState) -> AnyView {
    let reorder = Reorder::new(move |from, to| state.move_stat(from, to));
    view! {
        <section class="settings-section loadout-card" aria-labelledby="stats-heading">
            <h2 id="stats-heading" class="loadout-card-title">"Stat priority"</h2>
            <p class="loadout-hint">
                "Highest priority first. Drag a row, or focus its handle and use the arrow keys, to reorder. \
                 Leave a value blank to leave the stat out."
            </p>
            <div class="stat-list">
                <For
                    each=move || state.stats.get()
                    key=|row| row.key
                    children=move |row| view! { <StatLine state row reorder/> }
                />
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn StatLine(state: EditorState, row: StatRow, reorder: Reorder) -> AnyView {
    let index = move || {
        state
            .stats
            .with_untracked(|rows| rows.iter().position(|r| r.key == row.key))
            .unwrap_or(0)
    };
    let position = Memo::new(move |_| {
        state
            .stats
            .with(|rows| rows.iter().position(|r| r.key == row.key))
            .unwrap_or(0)
    });
    let handle_id = Memo::new(move |_| reorder.item_id(position.get()));

    view! {
        <div
            class="stat-row"
            class:drop-target=move || reorder.is_target(position.get())
            on:dragover=move |ev| reorder.over(index(), &ev)
            on:drop=move |ev| reorder.drop(index(), &ev)
        >
            <button
                type="button"
                class="stat-handle"
                id=move || handle_id.get()
                draggable="true"
                aria-label=move || format!("Move {}, priority {}", row.stat.get(), position.get() + 1)
                on:dragstart=move |ev| reorder.start(index(), &ev)
                on:dragend=move |_| reorder.end()
                on:keydown=move |ev| {
                    reorder.key(index(), state.stats.with_untracked(Vec::len), Axis::Column, false, &ev);
                }
            >
                <Icon name="grip"/>
            </button>
            <span class="stat-rank" aria-hidden="true">{move || position.get() + 1}</span>
            <EnumIcon label=row.stat/>
            <span class="stat-name">{move || row.stat.get()}</span>
            <TextInput
                id=format!("stat-value-{}", row.key)
                label="Value"
                value=row.value
                placeholder="0–200"
                kind="number"
            />
        </div>
    }
    .into_any()
}

#[component]
pub fn DetailsCard(state: EditorState) -> AnyView {
    view! {
        <section class="settings-section loadout-card" aria-labelledby="details-heading">
            <h2 id="details-heading" class="loadout-card-title">"Details"</h2>
            <div class="loadout-grid">
                <TextInput id="loadout-author" label="Author" value=state.author/>
                <TextInput id="loadout-dim" label="DIM link" value=state.dim_link placeholder="https://dim.gg/…" kind="url"/>
                <TextInput id="loadout-video" label="Video link" value=state.video_url placeholder="https://youtu.be/…" kind="url"/>
            </div>
            <KeyListField label="Tags (up to 3)" keys=state.tags list="" max=3/>
            <div class="setting-field">
                <label for="loadout-how">"How it works"</label>
                <textarea
                    id="loadout-how"
                    class="input"
                    rows="8"
                    prop:value=move || state.how_it_works.get()
                    on:input=move |ev| state.how_it_works.set(event_target_value(&ev))
                ></textarea>
            </div>
        </section>
    }
    .into_any()
}
