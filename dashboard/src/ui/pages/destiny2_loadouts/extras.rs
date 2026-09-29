use leptos::prelude::*;

use super::fields::{OptionSelect, TextInput};
use super::ranking::Field;
use super::slots::{EnumIcon, KeySlotList};
use super::state::{EditorState, StatRow};
use crate::ui::components::key_list::KeyListField;

#[component]
pub fn ArtifactCard(state: EditorState) -> AnyView {
    view! {
        <section class="settings-section loadout-card" aria-labelledby="artifact-heading">
            <h2 id="artifact-heading" class="loadout-card-title">"Artifact"</h2>
            <TextInput id="artifact-name" label="Artifact name" value=state.artifact_name/>
            <span class="label">"Artifact perks"</span>
            <KeySlotList label="Artifact perks" values=state.artifact_perks field=Field::ArtifactPerk max=12/>
        </section>
    }
    .into_any()
}

#[component]
pub fn StatsCard(state: EditorState, stats: Vec<String>) -> AnyView {
    let stats = StoredValue::new(stats);
    view! {
        <section class="settings-section loadout-card" aria-labelledby="stats-heading">
            <h2 id="stats-heading" class="loadout-card-title">"Stat priority"</h2>
            <p class="loadout-hint">"Highest priority first. Leave a value blank to leave the stat out."</p>
            <div class="stat-list">
                <For
                    each=move || state.stats.get()
                    key=|row| row.key
                    children=move |row| view! { <StatLine row stats=stats.get_value()/> }
                />
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn StatLine(row: StatRow, stats: Vec<String>) -> AnyView {
    view! {
        <div class="stat-row">
            <EnumIcon label=row.stat/>
            <OptionSelect id=format!("stat-{}", row.key) label="Stat" value=row.stat options=stats/>
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
