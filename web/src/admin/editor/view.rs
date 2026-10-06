use topcoat::Result;
use topcoat::asset::Asset;
use topcoat::view::{Unescaped, View, component, view};

use super::index::{CatalogIndex, missing_emojis};
use crate::admin::dto::{
    ArmourForm,
    AspectForm,
    LoadoutCatalog,
    LoadoutForm,
    StatForm,
    WeaponForm,
};
use crate::admin::keys::display_name;
use crate::components::icons::{Icon, icon};
use crate::components::key_list::key_list_field;

const MAX_ASPECTS: usize = 2;
const MAX_WEAPONS: usize = 3;
const MAX_FRAGMENTS: usize = 6;
const MAX_PERKS: usize = 5;
const MAX_MODS: usize = 5;
const MAX_ARTIFACT_PERKS: usize = 12;
const MAX_TAGS: usize = 3;

const REORDER_HINT: &str = "Drag, or press Alt + arrow keys, to reorder";

struct Lists {
    fragments: usize,
    perks: usize,
    mods: usize,
    artifact: usize,
    stats: usize,
}

impl Lists {
    const fn new(form: &LoadoutForm) -> Self {
        let fragments = 0;
        let perks = fragments + form.aspects.len();
        let mods = perks + form.weapons.len();
        let artifact = mods + form.armour.len();
        Self { fragments, perks, mods, artifact, stats: artifact + 1 }
    }
}

#[component]
pub(super) async fn editor(
    catalog: &LoadoutCatalog,
    form: &LoadoutForm,
    data: &str,
    script: Asset,
) -> Result<impl View> {
    let index = CatalogIndex::new(catalog);
    let missing = missing_emojis(&index, form);
    let images_down = !index.has_any_emoji();
    let heading = if form.id.is_some() { "Edit loadout" } else { "New loadout" };
    let o = &catalog.options;

    Ok(view! {
        <form class="loadout-editor">
            <header class="loadout-header">
                <h1>(heading)</h1>
                text_input(
                    id: "loadout-name",
                    label: "Build name",
                    value: &form.name,
                    placeholder: "",
                    kind: "text"
                )
                <div class="loadout-choices">
                    icon_choice(
                        label: "Class",
                        value: &form.class,
                        options: &o.classes,
                        index: &index
                    )
                    icon_choice(
                        label: "Subclass",
                        value: &form.element,
                        options: &o.elements,
                        index: &index
                    )
                    icon_choice(
                        label: "Mode",
                        value: &form.mode,
                        options: &o.modes,
                        index: &index
                    )
                </div>
                <div class="budget" role="status" aria-live="polite"></div>
            </header>
            if images_down {
                <p class="loadout-warning" role="status">
                    "Zayden's emoji list couldn't be loaded, so icons are hidden. You can still edit and save."
                </p>
            }
            if !missing.is_empty() {
                <p class="loadout-warning" role="status">
                    "Zayden has no emoji for these, so /destiny2 builds can't show this build until they're added: "
                    (missing.join(", "))
                </p>
            }
            <div class="loadout-board">
                subclass_card(form: form, index: &index)
                gear_card(form: form, catalog: catalog, index: &index)
                artifact_card(form: form, index: &index)
                stats_card(form: form, index: &index)
                details_card(form: form)
            </div>
            <div class="form-actions">
                <button type="submit" class="btn btn-primary">"Save"</button>
            </div>
        </form>
        <dialog class="picker" aria-labelledby="picker-title"></dialog>
        <script type="application/json" id="loadout-editor-data">
            (Unescaped::new_unchecked(data.to_owned()))
        </script>
        <script type="module" src=(script)></script>
    })
}

#[component]
async fn text_input(
    id: &str,
    label: &str,
    value: &str,
    placeholder: &str,
    kind: &str,
) -> Result<impl View> {
    Ok(view! {
        <div class="setting-field">
            <label for=(id)>(label)</label>
            <input
                id=(id)
                class="input"
                type=(kind)
                placeholder=(placeholder)
                value=(value)
            >
        </div>
    })
}

#[component]
async fn enum_icon(index: &CatalogIndex, label: &str) -> Result<impl View> {
    Ok(view! {
        if let Some(src) = index.enum_image(label) {
            <img class="enum-icon" src=(src) alt="">
        }
    })
}

#[component]
async fn icon_choice(
    label: &str,
    value: &str,
    options: &[String],
    index: &CatalogIndex,
) -> Result<impl View> {
    Ok(view! {
        <div class="icon-choice-field" role="radiogroup" aria-label=(label)>
            <span class="label">(label)</span>
            <div class="icon-choice">
                #[key(position)]
                for (position, option) in options.iter().enumerate() {
                    let checked = option == value;
                    <button
                        type="button"
                        role="radio"
                        class=(if checked {
                            "icon-choice-option active"
                        } else {
                            "icon-choice-option"
                        })
                        aria-checked=(if checked { "true" } else { "false" })
                    >
                        enum_icon(index: index, label: option)
                        <span>(option.as_str())</span>
                    </button>
                }
            </div>
        </div>
    })
}

#[component]
async fn slot_face(
    key: &str,
    label: &str,
    diamond: bool,
    id: Option<String>,
    index: &CatalogIndex,
) -> Result<impl View> {
    let filled = !key.is_empty();
    let class = match (diamond, filled) {
        (true, true) => "slot slot-diamond slot-filled",
        (true, false) => "slot slot-diamond",
        (false, true) => "slot slot-filled",
        (false, false) => "slot",
    };
    let aria = if filled {
        format!("{label}: {}, change", display_name(key))
    } else {
        format!("{label}: empty, choose one")
    };
    let tooltip = filled.then(|| format!("{} ({key})", display_name(key)));

    Ok(view! {
        <div class=(class)>
            <button
                type="button"
                class="slot-button"
                id=(id)
                aria-label=(aria)
                title=(tooltip)
            >
                if !filled {
                    <span class="slot-plus" aria-hidden="true">
                        icon(name: Icon::Plus)
                    </span>
                } else if let Some(src) = index.image(key) {
                    <img src=(src) alt="" loading="lazy">
                } else {
                    <span
                        class="slot-missing"
                        title=(format!("Zayden has no emoji named {key}"))
                    >
                        "?"
                    </span>
                }
            </button>
            if filled {
                <button
                    type="button"
                    class="slot-clear"
                    aria-label=(format!("Remove {label}"))
                >
                    icon(name: Icon::X)
                </button>
            }
        </div>
    })
}

#[component]
async fn key_slot(
    label: &str,
    value: &str,
    #[default] diamond: bool,
    index: &CatalogIndex,
) -> Result<impl View> {
    Ok(view! {
        <div class="slot-cell">
            slot_face(
                key: value,
                label: label,
                diamond: diamond,
                id: None,
                index: index
            )
            <span class="slot-caption">(label)</span>
        </div>
    })
}

#[component]
async fn key_slot_list(
    label: &str,
    noun: &str,
    values: &[String],
    max: usize,
    list: usize,
    #[default] round: bool,
    #[default] reorderable: bool,
    index: &CatalogIndex,
) -> Result<impl View> {
    let len = values.len();
    let slots: Vec<(usize, &str)> = values
        .iter()
        .map(String::as_str)
        .chain((len < max).then_some(""))
        .enumerate()
        .collect();

    Ok(view! {
        <div
            class=(if round { "slot-list slot-list-round" } else { "slot-list" })
            role="group"
            aria-label=(label)
        >
            #[key(position)]
            for (position, key) in slots {
                let face_label = format!("{label} {noun} {}", position + 1);
                let id = format!("reorder-{list}-{position}");
                if reorderable && !key.is_empty() {
                    <div class="slot-drag" draggable="true" title=(REORDER_HINT)>
                        slot_face(
                            key: key,
                            label: &face_label,
                            diamond: false,
                            id: Some(id),
                            index: index
                        )
                    </div>
                } else {
                    slot_face(
                        key: key,
                        label: &face_label,
                        diamond: false,
                        id: Some(id),
                        index: index
                    )
                }
            }
            <span class="slot-count" aria-hidden="true">(format!("{len}/{max}"))</span>
        </div>
    })
}

#[component]
async fn item_slot(
    label: &str,
    name: &str,
    icon_url: &str,
    #[default] clearable: bool,
) -> Result<impl View> {
    let filled = !name.is_empty();
    let aria = if filled {
        format!("{label}: {name}, change")
    } else {
        format!("{label}: empty, choose one")
    };

    Ok(view! {
        <div
            class=(if filled { "slot slot-item slot-filled" } else { "slot slot-item" })
        >
            <button type="button" class="slot-button" aria-label=(aria) title=(name)>
                if !filled {
                    <span class="slot-plus" aria-hidden="true">
                        icon(name: Icon::Plus)
                    </span>
                } else if icon_url.is_empty() {
                    <span class="slot-missing" aria-hidden="true">"?"</span>
                } else {
                    <img src=(icon_url) alt="" loading="lazy">
                }
            </button>
            if clearable && filled {
                <button
                    type="button"
                    class="slot-clear"
                    aria-label=(format!("Remove {label}"))
                >
                    icon(name: Icon::X)
                </button>
            }
        </div>
    })
}

#[component]
async fn subclass_card(
    form: &LoadoutForm,
    index: &CatalogIndex,
) -> Result<impl View> {
    let lists = Lists::new(form);

    Ok(view! {
        <section
            class="settings-section loadout-card"
            aria-labelledby="subclass-heading"
        >
            <h2 id="subclass-heading" class="loadout-card-title">"Subclass"</h2>
            <div class="subclass-top">
                <div class="super-field">
                    key_slot(
                        label: "Super",
                        value: &form.super_emoji,
                        diamond: true,
                        index: index
                    )
                </div>
                <div class="ability-row" role="group" aria-label="Abilities">
                    key_slot(
                        label: "Class ability",
                        value: &form.class_ability,
                        index: index
                    )
                    key_slot(label: "Jump", value: &form.jump, index: index)
                    key_slot(label: "Melee", value: &form.melee, index: index)
                    key_slot(label: "Grenade", value: &form.grenade, index: index)
                </div>
            </div>
            <div class="aspect-list">
                #[key(position)]
                for (position, row) in form.aspects.iter().enumerate() {
                    aspect_block(
                        row: row,
                        list: lists.fragments + position,
                        index: index
                    )
                }
                if form.aspects.len() < MAX_ASPECTS {
                    <button type="button" class="btn btn-secondary">
                        icon(name: Icon::Plus)
                        "Add aspect"
                    </button>
                }
            </div>
        </section>
    })
}

#[component]
async fn aspect_block(
    row: &AspectForm,
    list: usize,
    index: &CatalogIndex,
) -> Result<impl View> {
    Ok(view! {
        <div class="aspect-block">
            key_slot(label: "Aspect", value: &row.aspect, index: index)
            <div class="aspect-fragments">
                <span class="label">"Fragments"</span>
                key_slot_list(
                    label: "Fragments",
                    noun: "fragment",
                    values: &row.fragments,
                    max: MAX_FRAGMENTS,
                    list: list,
                    index: index
                )
            </div>
            <button type="button" class="btn btn-ghost aspect-remove">
                "Remove aspect"
            </button>
        </div>
    })
}

#[component]
async fn gear_card(
    form: &LoadoutForm,
    catalog: &LoadoutCatalog,
    index: &CatalogIndex,
) -> Result<impl View> {
    let lists = Lists::new(form);
    let weapons = form.weapons.len();

    Ok(view! {
        <section class="settings-section loadout-card" aria-labelledby="gear-heading">
            <h2 id="gear-heading" class="loadout-card-title">"Gear and mods"</h2>
            <h3 class="loadout-subtitle">"Weapons"</h3>
            <div class="gear-list">
                #[key(position)]
                for (position, row) in form.weapons.iter().enumerate() {
                    weapon_line(
                        row: row,
                        catalog: catalog,
                        list: lists.perks + position,
                        index: index
                    )
                }
                if weapons < MAX_WEAPONS {
                    <div class="gear-row">
                        item_slot(label: "New weapon", name: "", icon_url: "")
                        <div class="gear-info">
                            <span class="gear-name gear-empty">"Add a weapon"</span>
                            <span class="gear-meta">
                                (format!("{weapons} of {MAX_WEAPONS}"))
                            </span>
                        </div>
                    </div>
                }
            </div>
            <h3 class="loadout-subtitle">"Armour"</h3>
            <p class="loadout-hint">
                "Leave a slot empty to leave it out of the build."
            </p>
            <div class="gear-list">
                #[key(position)]
                for (position, row) in form.armour.iter().enumerate() {
                    armour_line(row: row, list: lists.mods + position, index: index)
                }
            </div>
        </section>
    })
}

#[component]
async fn weapon_line(
    row: &WeaponForm,
    catalog: &LoadoutCatalog,
    list: usize,
    index: &CatalogIndex,
) -> Result<impl View> {
    let hint = catalog
        .weapons
        .iter()
        .find(|w| w.name == row.name)
        .map(|w| w.affinity.as_str())
        .filter(|usual| *usual != row.affinity);

    Ok(view! {
        <div class="gear-row">
            item_slot(label: "Weapon", name: &row.name, icon_url: &row.icon_url)
            <div class="gear-info">
                <span class="gear-name">(row.name.as_str())</span>
                <span class="gear-meta">
                    <button
                        type="button"
                        class="gear-affinity"
                        aria-expanded="false"
                        aria-label=(format!("Damage type: {}, change", row.affinity))
                    >
                        enum_icon(index: index, label: &row.affinity)
                        (row.affinity.as_str())
                    </button>
                    (row.archetype.as_str())
                    if let Some(usual) = hint {
                        <span class="gear-hint">(format!("Normally {usual}"))</span>
                    }
                </span>
            </div>
            key_slot_list(
                label: "Perks",
                noun: "perk",
                values: &row.perks,
                max: MAX_PERKS,
                list: list,
                round: true,
                index: index
            )
            <button
                type="button"
                class="btn btn-ghost gear-remove"
                aria-label=(format!("Remove {}", row.name))
            >
                icon(name: Icon::X)
            </button>
        </div>
    })
}

#[component]
async fn armour_line(
    row: &ArmourForm,
    list: usize,
    index: &CatalogIndex,
) -> Result<impl View> {
    let empty = row.name.is_empty();

    Ok(view! {
        <div class="gear-row">
            item_slot(
                label: &row.slot,
                name: &row.name,
                icon_url: &row.icon_url,
                clearable: true
            )
            <div class="gear-info">
                <span class=(if empty { "gear-name gear-empty" } else { "gear-name" })>
                    (if empty { "Empty" } else { row.name.as_str() })
                </span>
                <span class="gear-meta">
                    enum_icon(index: index, label: &row.slot)
                    (row.slot.as_str())
                </span>
            </div>
            key_slot_list(
                label: "Mods",
                noun: "mod",
                values: &row.mods,
                max: MAX_MODS,
                list: list,
                reorderable: true,
                index: index
            )
        </div>
    })
}

#[component]
async fn artifact_card(
    form: &LoadoutForm,
    index: &CatalogIndex,
) -> Result<impl View> {
    let lists = Lists::new(form);
    let name = form.artifact_name.as_str();
    let named = !name.is_empty();
    let aria = if named {
        format!("Artifact: {name}, change")
    } else {
        "Artifact: none, choose one".to_owned()
    };

    Ok(view! {
        <section
            class="settings-section loadout-card"
            aria-labelledby="artifact-heading"
        >
            <h2 id="artifact-heading" class="loadout-card-title">"Artifact"</h2>
            <div class="artifact-row">
                <button
                    type="button"
                    class=(if named {
                        "artifact-choice"
                    } else {
                        "artifact-choice artifact-empty"
                    })
                    aria-label=(aria)
                >
                    (if named { name } else { "Choose artifact" })
                </button>
                if named {
                    <button type="button" class="btn btn-ghost">"Clear"</button>
                }
            </div>
            <span class="label">"Artifact perks"</span>
            key_slot_list(
                label: "Artifact perks",
                noun: "artifact perk",
                values: &form.artifact_perks,
                max: MAX_ARTIFACT_PERKS,
                list: lists.artifact,
                reorderable: true,
                index: index
            )
        </section>
    })
}

#[component]
async fn stats_card(form: &LoadoutForm, index: &CatalogIndex) -> Result<impl View> {
    let lists = Lists::new(form);

    Ok(view! {
        <section class="settings-section loadout-card" aria-labelledby="stats-heading">
            <h2 id="stats-heading" class="loadout-card-title">"Stat priority"</h2>
            <p class="loadout-hint">
                "Highest priority first. Drag a row, or focus its handle and use the arrow keys, to reorder. Leave a value blank to leave the stat out."
            </p>
            <div class="stat-list">
                #[key(position)]
                for (position, row) in form.stats.iter().enumerate() {
                    stat_line(
                        row: row,
                        position: position,
                        list: lists.stats,
                        index: index
                    )
                }
            </div>
        </section>
    })
}

#[component]
async fn stat_line(
    row: &StatForm,
    position: usize,
    list: usize,
    index: &CatalogIndex,
) -> Result<impl View> {
    let rank = position + 1;
    let input_id = format!("stat-value-{position}");

    Ok(view! {
        <div class="stat-row">
            <button
                type="button"
                class="stat-handle"
                id=(format!("reorder-{list}-{position}"))
                draggable="true"
                aria-label=(format!("Move {}, priority {rank}", row.stat))
            >
                icon(name: Icon::Grip)
            </button>
            <span class="stat-rank" aria-hidden="true">(rank)</span>
            enum_icon(index: index, label: &row.stat)
            <span class="stat-name">(row.stat.as_str())</span>
            text_input(
                id: &input_id,
                label: "Value",
                value: &row.value,
                placeholder: "0\u{2013}200",
                kind: "number"
            )
        </div>
    })
}

#[component]
async fn details_card(form: &LoadoutForm) -> Result<impl View> {
    Ok(view! {
        <section
            class="settings-section loadout-card"
            aria-labelledby="details-heading"
        >
            <h2 id="details-heading" class="loadout-card-title">"Details"</h2>
            <div class="loadout-grid">
                text_input(
                    id: "loadout-author",
                    label: "Author",
                    value: &form.author,
                    placeholder: "",
                    kind: "text"
                )
                text_input(
                    id: "loadout-dim",
                    label: "DIM link",
                    value: &form.dim_link,
                    placeholder: "https://dim.gg/\u{2026}",
                    kind: "url"
                )
                text_input(
                    id: "loadout-video",
                    label: "Video link",
                    value: &form.video_url,
                    placeholder: "https://youtu.be/\u{2026}",
                    kind: "url"
                )
            </div>
            key_list_field(
                label: "Tags (up to 3)",
                keys: &form.tags,
                list: "",
                max: MAX_TAGS
            )
            <div class="setting-field">
                <label for="loadout-how">"How it works"</label>
                <textarea id="loadout-how" class="input" rows="8">
                    "\n"
                    (form.how_it_works.as_str())
                </textarea>
            </div>
        </section>
    })
}
