use leptos::prelude::*;

use super::fields::OptionSelect;
use super::picker::{Picked, PickerCtx};
use super::ranking::Field;
use crate::dto::destiny2::{CatalogWeaponInfo, EmojiSource};
use crate::dto::destiny2_keys::{display_name, is_valid_key, key_from_query};
use crate::server::destiny2::CreateZaydenEmoji;

#[cfg(feature = "hydrate")]
const MAX_IMAGE_BYTES: f64 = 256.0 * 1024.0;

fn server_message(e: &ServerFnError) -> String {
    if let ServerFnError::ServerError(message) = e {
        message.clone()
    } else {
        e.to_string()
    }
}

fn https_preview(url: &str) -> Option<String> {
    let url = url.trim();
    url.starts_with("https://").then(|| url.to_owned())
}

#[component]
pub fn CreatePanel(
    field: Field,
    query: String,
    on_back: Callback<()>,
    on_done: Callback<Picked>,
) -> AnyView {
    let form = match field {
        Field::Weapon => view! { <WeaponCreate query on_done/> }.into_any(),
        Field::Armour => view! { <ArmourCreate query on_done/> }.into_any(),
        Field::Artifact => view! { <ArtifactCreate query on_done/> }.into_any(),
        Field::Super
        | Field::ClassAbility
        | Field::Jump
        | Field::Melee
        | Field::Grenade
        | Field::Aspect
        | Field::Fragment
        | Field::WeaponPerk
        | Field::ArmourMod
        | Field::ArtifactPerk => view! { <EmojiCreate query on_done/> }.into_any(),
    };
    view! {
        <div class="picker-create-form">
            <button type="button" class="btn btn-ghost picker-back" on:click=move |_| on_back.run(())>
                "Back to results"
            </button>
            {form}
        </div>
    }
    .into_any()
}

#[component]
fn Preview(#[prop(into)] src: Signal<Option<String>>) -> impl IntoView {
    move || {
        src.get().map(|src| {
            view! {
                <figure class="picker-preview">
                    <img src=src alt=""/>
                    <figcaption>"Preview"</figcaption>
                </figure>
            }
        })
    }
}

#[cfg(feature = "hydrate")]
fn read_image(
    ev: &leptos::ev::Event,
    file: RwSignal<Option<String>>,
    error: RwSignal<Option<String>>,
) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let Some(input) =
        ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return;
    };
    let Some(chosen) = input.files().and_then(|files| files.get(0)) else {
        file.set(None);
        return;
    };
    if chosen.size() > MAX_IMAGE_BYTES {
        file.set(None);
        error.set(Some(
            "That file is over 256 KiB. Choose a smaller image.".to_owned(),
        ));
        return;
    }
    let Ok(reader) = web_sys::FileReader::new() else {
        error.set(Some("This browser can't read files.".to_owned()));
        return;
    };
    let done = reader.clone();
    let onload = Closure::once_into_js(move || {
        match done.result().ok().and_then(|v| v.as_string()) {
            Some(uri) => {
                error.set(None);
                file.set(Some(uri));
            },
            None => error.set(Some("That file couldn't be read.".to_owned())),
        }
    });
    reader.set_onload(Some(onload.unchecked_ref()));
    if reader.read_as_data_url(&chosen).is_err() {
        error.set(Some("That file couldn't be read.".to_owned()));
    }
}

#[cfg(not(feature = "hydrate"))]
const fn read_image(
    _ev: &leptos::ev::Event,
    _file: RwSignal<Option<String>>,
    _error: RwSignal<Option<String>>,
) {
}

#[component]
fn EmojiCreate(query: String, on_done: Callback<Picked>) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let key = RwSignal::new(key_from_query(&query));
    let from_file = RwSignal::new(false);
    let url = RwSignal::new(String::new());
    let file = RwSignal::new(None::<String>);
    let file_error = RwSignal::new(None::<String>);
    let create = ServerAction::<CreateZaydenEmoji>::new();

    let key_problem = move || {
        let k = key.get();
        ctx.index.with(|idx| {
            if !is_valid_key(&k) {
                Some("Use 2–32 lowercase letters, digits or underscores.")
            } else if idx.is_reserved(&k) {
                Some("That name belongs to a built-in class, element, weapon or stat icon.")
            } else if idx.has_emoji(&k) {
                Some("Zayden already has this emoji. Go back and pick it from the list.")
            } else {
                None
            }
        })
    };
    let source = move || {
        if from_file.get() {
            file.get().map(EmojiSource::DataUri)
        } else {
            let u = url.get().trim().to_owned();
            (!u.is_empty()).then_some(EmojiSource::Url(u))
        }
    };
    let preview = Signal::derive(move || {
        if from_file.get() { file.get() } else { https_preview(&url.get()) }
    });
    let blocked = move || {
        create.pending().get() || key_problem().is_some() || source().is_none()
    };

    Effect::new(move |_| {
        if let Some(Ok(info)) = create.value().get() {
            ctx.catalog.update(|c| c.emojis.push(info.clone()));
            on_done.run(Picked::Key(info.name));
        }
    });

    view! {
        <div class="setting-field">
            <label for="new-emoji-name">"Emoji name"</label>
            <input
                id="new-emoji-name"
                class="input"
                autocomplete="off"
                spellcheck="false"
                aria-describedby="new-emoji-name-hint"
                aria-invalid=move || key_problem().is_some().to_string()
                prop:value=move || key.get()
                on:input=move |ev| key.set(event_target_value(&ev))
            />
            <p id="new-emoji-name-hint" class="field-hint" class:field-error=move || key_problem().is_some()>
                {move || key_problem().map_or_else(
                    || format!("Shows as “{}”.", display_name(&key.get())),
                    str::to_owned,
                )}
            </p>
        </div>
        <div class="segmented" role="tablist" aria-label="Image source">
            <button
                type="button"
                role="tab"
                aria-selected=move || (!from_file.get()).to_string()
                class=move || if from_file.get() { "seg" } else { "seg active" }
                on:click=move |_| from_file.set(false)
            >
                "Image link"
            </button>
            <button
                type="button"
                role="tab"
                aria-selected=move || from_file.get().to_string()
                class=move || if from_file.get() { "seg active" } else { "seg" }
                on:click=move |_| from_file.set(true)
            >
                "Upload file"
            </button>
        </div>
        <div class="setting-field">
            <Show
                when=move || from_file.get()
                fallback=move || view! {
                    <label for="new-emoji-url">"Image link"</label>
                    <input
                        id="new-emoji-url"
                        class="input"
                        type="url"
                        inputmode="url"
                        placeholder="https://…"
                        aria-describedby="new-emoji-image-hint"
                        prop:value=move || url.get()
                        on:input=move |ev| url.set(event_target_value(&ev))
                    />
                }
            >
                <label for="new-emoji-file">"Image file"</label>
                <input
                    id="new-emoji-file"
                    class="input"
                    type="file"
                    accept="image/png,image/jpeg,image/gif,image/webp"
                    aria-describedby="new-emoji-image-hint"
                    on:change=move |ev| read_image(&ev, file, file_error)
                />
            </Show>
            <p id="new-emoji-image-hint" class="field-hint">
                "PNG, JPEG, GIF or WebP, up to 256 KiB. Square images look best."
            </p>
        </div>
        <Preview src=preview/>
        {move || file_error.get().map(|e| view! { <p class="alert error" role="alert">{e}</p> })}
        {move || match create.value().get() {
            Some(Err(e)) => Some(view! { <p class="alert error" role="alert">{server_message(&e)}</p> }),
            _ => None,
        }}
        <div class="picker-actions">
            <button
                type="button"
                class="btn btn-primary"
                disabled=blocked
                on:click=move |_| {
                    if let Some(source) = source() {
                        create.dispatch(CreateZaydenEmoji { name: key.get_untracked(), source });
                    }
                }
            >
                {move || if create.pending().get() { "Creating…" } else { "Create emoji" }}
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn WeaponCreate(query: String, on_done: Callback<Picked>) -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let (affinities, archetypes) = ctx.catalog.with_untracked(|c| {
        (c.options.affinities.clone(), c.options.archetypes.clone())
    });
    let name = RwSignal::new(query.trim().to_owned());
    let affinity = RwSignal::new(affinities.first().cloned().unwrap_or_default());
    let archetype = RwSignal::new(archetypes.first().cloned().unwrap_or_default());
    let icon_url = RwSignal::new(String::new());
    let preview = Signal::derive(move || https_preview(&icon_url.get()));
    let blocked =
        move || name.with(|n| n.trim().is_empty()) || preview.with(Option::is_none);

    view! {
        <p class="field-hint">"Saved to the weapon catalog when you save this loadout."</p>
        <div class="setting-field">
            <label for="new-weapon-name">"Weapon name"</label>
            <input
                id="new-weapon-name"
                class="input"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />
        </div>
        <OptionSelect id="new-weapon-affinity" label="Damage type" value=affinity options=affinities/>
        <OptionSelect id="new-weapon-archetype" label="Weapon type" value=archetype options=archetypes/>
        <IconUrlField id="new-weapon-icon" value=icon_url/>
        <Preview src=preview/>
        <div class="picker-actions">
            <button
                type="button"
                class="btn btn-primary"
                disabled=blocked
                on:click=move |_| on_done.run(Picked::Weapon(CatalogWeaponInfo {
                    name: name.get_untracked().trim().to_owned(),
                    affinity: affinity.get_untracked(),
                    archetype: archetype.get_untracked(),
                    icon_url: icon_url.get_untracked().trim().to_owned(),
                    known_perks: Vec::new(),
                }))
            >
                "Use this weapon"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn ArmourCreate(query: String, on_done: Callback<Picked>) -> AnyView {
    let name = RwSignal::new(query.trim().to_owned());
    let icon_url = RwSignal::new(String::new());
    let preview = Signal::derive(move || https_preview(&icon_url.get()));
    let blocked =
        move || name.with(|n| n.trim().is_empty()) || preview.with(Option::is_none);

    view! {
        <div class="setting-field">
            <label for="new-armour-name">"Armour name"</label>
            <input
                id="new-armour-name"
                class="input"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />
        </div>
        <IconUrlField id="new-armour-icon" value=icon_url/>
        <Preview src=preview/>
        <div class="picker-actions">
            <button
                type="button"
                class="btn btn-primary"
                disabled=blocked
                on:click=move |_| on_done.run(Picked::Armour {
                    name: name.get_untracked().trim().to_owned(),
                    icon_url: icon_url.get_untracked().trim().to_owned(),
                })
            >
                "Use this armour"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn ArtifactCreate(query: String, on_done: Callback<Picked>) -> AnyView {
    let name = RwSignal::new(query.trim().to_owned());
    let blocked = move || name.with(|n| n.trim().is_empty());

    view! {
        <div class="setting-field">
            <label for="new-artifact-name">"Artifact name"</label>
            <input
                id="new-artifact-name"
                class="input"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />
        </div>
        <div class="picker-actions">
            <button
                type="button"
                class="btn btn-primary"
                disabled=blocked
                on:click=move |_| on_done.run(Picked::Key(name.get_untracked().trim().to_owned()))
            >
                "Use this artifact"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn IconUrlField(id: &'static str, value: RwSignal<String>) -> AnyView {
    view! {
        <div class="setting-field">
            <label for=id>"Icon link"</label>
            <input
                id=id
                class="input"
                type="url"
                inputmode="url"
                placeholder="https://www.bungie.net/common/destiny2_content/icons/…"
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            <p class="field-hint">"An https link to the item's icon, e.g. from light.gg or DIM."</p>
        </div>
    }
    .into_any()
}
