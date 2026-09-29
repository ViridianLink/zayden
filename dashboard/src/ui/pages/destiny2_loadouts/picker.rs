use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlDialogElement, HtmlElement};

use super::create::CreatePanel;
use super::ranking::{Candidate, CatalogIndex, Field, Scope, sections};
use crate::dto::destiny2::{CatalogWeaponInfo, LoadoutCatalog};
use crate::ui::components::icons::Icon;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    Key(String),
    Weapon(CatalogWeaponInfo),
    Armour { name: String, icon_url: String },
}

#[derive(Clone)]
pub struct PickRequest {
    pub field: Field,
    pub title: String,
    pub weapon: Option<String>,
    pub armour_slot: Option<String>,
    pub taken: Vec<String>,
    pub on_pick: Callback<Picked>,
}

#[derive(Clone, Copy)]
pub struct PickerCtx {
    pub request: RwSignal<Option<PickRequest>>,
    pub catalog: RwSignal<LoadoutCatalog>,
    pub index: Memo<CatalogIndex>,
    pub class: RwSignal<String>,
    pub element: RwSignal<String>,
}

impl PickerCtx {
    #[must_use]
    pub fn new(
        catalog: LoadoutCatalog,
        class: RwSignal<String>,
        element: RwSignal<String>,
    ) -> Self {
        let catalog = RwSignal::new(catalog);
        Self {
            request: RwSignal::new(None),
            catalog,
            index: Memo::new(move |_| catalog.with(CatalogIndex::new)),
            class,
            element,
        }
    }

    pub fn open(&self, request: PickRequest) {
        self.request.set(Some(request));
    }

    pub fn close(&self) {
        self.request.set(None);
    }

    #[must_use]
    pub fn image(&self, key: &str) -> Option<String> {
        self.index.with(|idx| idx.image(key))
    }
}

fn option_id(i: usize) -> String {
    format!("picker-option-{i}")
}

fn resolve(ctx: PickerCtx, field: Field, key: String) -> Option<Picked> {
    ctx.catalog.with_untracked(|c| match field {
        Field::Weapon => {
            c.weapons.iter().find(|w| w.name == key).cloned().map(Picked::Weapon)
        },
        Field::Armour => c.armour.iter().find(|a| a.name == key).map(|a| {
            Picked::Armour { name: a.name.clone(), icon_url: a.icon_url.clone() }
        }),
        Field::Super
        | Field::ClassAbility
        | Field::Jump
        | Field::Melee
        | Field::Grenade
        | Field::Aspect
        | Field::Fragment
        | Field::WeaponPerk
        | Field::ArmourMod
        | Field::ArtifactPerk
        | Field::Artifact => Some(Picked::Key(key)),
    })
}

#[component]
pub fn Picker() -> AnyView {
    let ctx = expect_context::<PickerCtx>();
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let search = NodeRef::<leptos::html::Input>::new();
    let query = RwSignal::new(String::new());
    let active = RwSignal::new(0_usize);
    let creating = RwSignal::new(false);
    let opener = StoredValue::new_local(None::<HtmlElement>);

    Effect::new(move |_| {
        let open = ctx.request.with(Option::is_some);
        let Some(d) = dialog.get() else { return };
        if open && !d.open() {
            opener.set_value(
                document()
                    .active_element()
                    .and_then(|e| e.dyn_into::<HtmlElement>().ok()),
            );
            query.set(String::new());
            active.set(0);
            creating.set(false);
            if d.show_modal().is_ok() {
                request_animation_frame(move || {
                    if let Some(input) = search.get_untracked() {
                        let _ = input.focus();
                    }
                });
            }
        } else if !open && d.open() {
            d.close();
        }
    });

    let results = Memo::new(move |_| {
        ctx.request.with(|r| {
            let Some(r) = r else { return Vec::new() };
            let class = ctx.class.get();
            let element = ctx.element.get();
            let scope = Scope {
                field: r.field,
                class: &class,
                element: &element,
                weapon: r.weapon.as_deref(),
                armour_slot: r.armour_slot.as_deref(),
                taken: &r.taken,
            };
            ctx.catalog.with(|c| {
                ctx.index.with(|idx| sections(c, idx, &scope, &query.get()))
            })
        })
    });
    let flat = Memo::new(move |_| {
        results.with(|s| {
            s.iter().flat_map(|s| s.items.clone()).collect::<Vec<Candidate>>()
        })
    });

    let pick = move |key: String| {
        let Some(req) = ctx.request.get_untracked() else { return };
        if let Some(picked) = resolve(ctx, req.field, key) {
            req.on_pick.run(picked);
        }
        ctx.close();
    };
    let choose = move |i: usize| {
        let item = flat.with_untracked(|f| f.get(i).cloned());
        match item {
            Some(c) if !c.selected => pick(c.key),
            Some(_) => {},
            None => creating.set(true),
        }
    };

    Effect::new(move |_| {
        let i = active.get();
        if let Some(el) = document().get_element_by_id(&option_id(i)) {
            el.scroll_into_view_with_bool(false);
        }
    });

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let last = flat.with_untracked(Vec::len);
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                active.update(|a| *a = (*a + 1).min(last));
            },
            "ArrowUp" => {
                ev.prevent_default();
                active.update(|a| *a = a.saturating_sub(1));
            },
            "Enter" => {
                ev.prevent_default();
                choose(active.get_untracked());
            },
            _ => {},
        }
    };

    let title = move || ctx.request.with(|r| r.as_ref().map(|r| r.title.clone()));
    let noun =
        move || ctx.request.with(|r| r.as_ref().map_or("item", |r| r.field.noun()));

    view! {
        <dialog
            node_ref=dialog
            class="picker"
            aria-labelledby="picker-title"
            on:close=move |_| {
                ctx.close();
                if let Some(el) = opener.get_value() {
                    let _ = el.focus();
                }
            }
            on:click=move |ev| {
                let on_backdrop = ev
                    .target()
                    .is_some_and(|t| t.dyn_into::<HtmlDialogElement>().is_ok());
                if on_backdrop {
                    ctx.close();
                }
            }
        >
            <Show when=move || ctx.request.with(Option::is_some)>
            <div class="picker-panel">
                <header class="picker-header">
                    <h2 id="picker-title" class="picker-title">{title}</h2>
                    <button
                        type="button"
                        class="icon-button"
                        aria-label="Close"
                        on:click=move |_| ctx.close()
                    >
                        <Icon name="x"/>
                    </button>
                </header>
                <Show
                    when=move || creating.get()
                    fallback=move || view! {
                        <div class="picker-search">
                            <input
                                node_ref=search
                                class="input"
                                type="search"
                                role="combobox"
                                aria-expanded="true"
                                aria-controls="picker-results"
                                aria-activedescendant=move || option_id(active.get())
                                aria-label=move || format!("Search {}s", noun())
                                placeholder=move || format!("Search {}s…", noun())
                                prop:value=move || query.get()
                                on:input=move |ev| {
                                    query.set(event_target_value(&ev));
                                    active.set(0);
                                }
                                on:keydown=on_keydown
                            />
                        </div>
                        <ul id="picker-results" class="picker-results" role="listbox">
                            <For
                                each=move || {
                                    let mut offset = 0_usize;
                                    results.get().into_iter().map(|section| {
                                        let start = offset;
                                        offset += section.items.len();
                                        (start, section)
                                    }).collect::<Vec<_>>()
                                }
                                key=|row| row.clone()
                                children=move |(start, section)| view! {
                                    <li role="presentation" class="picker-section">
                                        <span class="picker-section-title">{section.title}</span>
                                        <ul role="group" class="picker-group">
                                            {section.items.into_iter().enumerate().map(|(n, c)| {
                                                view! { <PickerOption c index=start + n active choose/> }
                                            }).collect_view()}
                                        </ul>
                                    </li>
                                }
                            />
                            <li
                                id=move || option_id(flat.with(Vec::len))
                                role="option"
                                aria-selected=move || (active.get() == flat.with(Vec::len)).to_string()
                                class="picker-option picker-create"
                                class:active=move || active.get() == flat.with(Vec::len)
                                on:click=move |_| creating.set(true)
                            >
                                <span class="picker-thumb"><Icon name="plus"/></span>
                                <span class="picker-label">
                                    {move || {
                                        let q = query.get();
                                        if q.trim().is_empty() {
                                            format!("Create new {}…", noun())
                                        } else {
                                            format!("Create new “{}”…", q.trim())
                                        }
                                    }}
                                </span>
                            </li>
                        </ul>
                    }
                >
                    {move || ctx.request.get().map(|r| view! {
                        <CreatePanel
                            field=r.field
                            query=query.get_untracked()
                            on_back=Callback::new(move |()| creating.set(false))
                            on_done=Callback::new(move |picked: Picked| {
                                if let Some(req) = ctx.request.get_untracked() {
                                    req.on_pick.run(picked);
                                }
                                ctx.close();
                            })
                        />
                    })}
                </Show>
            </div>
            </Show>
        </dialog>
    }
    .into_any()
}

#[component]
fn PickerOption(
    c: Candidate,
    index: usize,
    active: RwSignal<usize>,
    choose: impl Fn(usize) + Copy + Send + Sync + 'static,
) -> AnyView {
    let Candidate { label, detail, image, selected, .. } = c;
    view! {
        <li
            id=option_id(index)
            role="option"
            aria-selected=move || (active.get() == index).to_string()
            aria-disabled=selected.to_string()
            class="picker-option"
            class:active=move || active.get() == index
            class:taken=selected
            on:click=move |_| choose(index)
            on:mousemove=move |_| active.set(index)
        >
            <span class="picker-thumb">
                {image.map_or_else(
                    || view! { <span class="slot-missing" aria-hidden="true">"?"</span> }.into_any(),
                    |src| view! { <img src=src alt="" loading="lazy"/> }.into_any(),
                )}
            </span>
            <span class="picker-label">{label}</span>
            <span class="picker-detail">{detail}</span>
            {selected.then(|| view! { <span class="picker-tag">"Selected"</span> })}
        </li>
    }
    .into_any()
}
