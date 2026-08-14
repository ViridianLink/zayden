use std::collections::HashMap;

use leptos::prelude::*;
use leptos_meta::Title;

use crate::dto::{PalEdit, PlayerEdit, SaveEdits, SavePal};
use crate::server::palworld_save::get_save_roster;
use crate::ui::components::layout::AppShell;
use crate::ui::pages::not_found::NotFound;

const MAX_TRAITS: usize = 4;

const MAX_LEVEL: i64 = 80;
const MAX_TALENT: i64 = 100;

const STATUS_POINTS_PER_LEVEL: i32 = 1;
const TECH_POINTS_PER_LEVEL: i32 = 6;

const FALLBACK_EXPORT_NAME: &str = "Level_modified.sav";

#[component]
pub(crate) fn PalworldSavePage() -> impl IntoView {
    let roster = Resource::new(|| (), |()| get_save_roster());

    let pending: RwSignal<HashMap<String, PalEdit>> = RwSignal::new(HashMap::new());
    let player_pending: RwSignal<HashMap<String, PlayerEdit>> =
        RwSignal::new(HashMap::new());
    let (status, set_status) = signal(String::new());

    let export = move |_| {
        let edits = SaveEdits {
            player_edits: player_pending.get_untracked().into_values().collect(),
            pal_edits: pending.get_untracked().into_values().collect(),
        };
        set_status.set("Building save\u{2026}".to_string());
        leptos::task::spawn_local(async move {
            match download_export(&edits).await {
                Ok(name) => set_status.set(format!("Downloaded {name}.")),
                Err(e) => set_status.set(format!("Export failed: {e}")),
            }
        });
    };

    view! {
        <Title text="Save editor - Zayden"/>
        <Suspense fallback=|| view! { <p class="loading">"Loading world\u{2026}"</p> }>
            {move || roster.get().map(|result| match result {
                Err(_) => view! { <NotFound/> }.into_any(),
                Ok(view_model) => {
                    let labels: HashMap<String, String> =
                        view_model.trait_labels.iter().cloned().collect();
                    let options: Vec<(String, String)> = view_model
                        .roster
                        .trait_ids
                        .iter()
                        .map(|t| {
                            let label = labels
                                .get(t)
                                .cloned()
                                .unwrap_or_else(|| t.clone());
                            (t.clone(), label)
                        })
                        .collect();
                    let modified = view_model.roster.level_modified;

                    view! {
                        <AppShell>
                            <div class="page save-editor">
                                <div class="page-header">
                                    <div>
                                        <h1>"Save editor"</h1>
                                        <p class="page-lead">
                                            "Edits are applied to a copy and handed back as a download. "
                                            "Nothing is written to the game server. "
                                            "Palbox pals are stored in a separate file and are not listed here. "
                                            "Changing a character level also grants the status and technology "
                                            "points that level would have earned, so the export becomes a zip: "
                                            "unpack it over the world directory, since Level.sav and the player "
                                            "file are only valid together."
                                        </p>
                                        <p class="save-meta">
                                            "Level.sav modified: " {modified}
                                        </p>
                                    </div>
                                </div>

                                {view_model.roster.players.into_iter().map(|player| {
                                    let pid = player.instance_id.clone();
                                    let grant_id = player.instance_id.clone();
                                    let level = player.level;
                                    view! {
                                        <details class="save-group" open=true>
                                            <summary>
                                                {player.name.clone()}
                                                <span class="save-count">
                                                    {format!(" - {} pals", player.pals.len())}
                                                </span>
                                            </summary>
                                            <label class="save-field">
                                                {format!("Character level (1-{MAX_LEVEL})")}
                                                <input
                                                    type="number" min="1" max=MAX_LEVEL
                                                    value=level
                                                    on:change=move |ev| {
                                                        let id = pid.clone();
                                                        let Ok(v) = event_target_value(&ev).parse::<i64>() else {
                                                            player_pending.update(|m| { let _ = m.remove(&id); });
                                                            return;
                                                        };
                                                        let v = v.clamp(1, MAX_LEVEL);
                                                        set_input_value(&ev, v);
                                                        player_pending.update(|m| {
                                                            let _ = m.insert(id.clone(), PlayerEdit {
                                                                instance_id: id,
                                                                level: i32::try_from(v).ok(),
                                                            });
                                                        });
                                                    }
                                                />
                                            </label>
                                            <p class="save-grant">
                                                {move || grant_summary(
                                                    level,
                                                    player_pending
                                                        .get()
                                                        .get(&grant_id)
                                                        .and_then(|e| e.level),
                                                )}
                                            </p>
                                            {player.pals.into_iter().map(|pal| {
                                                pal_row(pal, pending, options.clone())
                                            }).collect_view()}
                                        </details>
                                    }
                                }).collect_view()}

                                <details class="save-group">
                                    <summary>
                                        "Base and unowned pals"
                                        <span class="save-count">
                                            {format!(" - {}", view_model.roster.base_pals.len())}
                                        </span>
                                    </summary>
                                    {view_model.roster.base_pals.into_iter().map(|pal| {
                                        pal_row(pal, pending, options.clone())
                                    }).collect_view()}
                                </details>

                                <div class="save-footer">
                                    <span class="save-pending">
                                        {move || format!(
                                            "{} pending edit(s)",
                                            pending.get().len() + player_pending.get().len(),
                                        )}
                                    </span>
                                    <button
                                        type="button" class="btn btn-secondary"
                                        on:click=move |_| {
                                            pending.set(HashMap::new());
                                            player_pending.set(HashMap::new());
                                            set_status.set(String::new());
                                        }
                                    >"Discard"</button>
                                    <button
                                        type="button" class="btn btn-primary"
                                        on:click=export
                                    >"Export save"</button>
                                    <span class="save-status">{move || status.get()}</span>
                                </div>
                            </div>
                        </AppShell>
                    }.into_any()
                },
            })}
        </Suspense>
    }
}

fn pal_row(
    pal: SavePal,
    pending: RwSignal<HashMap<String, PalEdit>>,
    options: Vec<(String, String)>,
) -> impl IntoView {
    let SavePal {
        instance_id,
        species,
        nickname,
        gender,
        stars,
        is_lucky,
        is_alpha,
        level,
        talent_hp,
        talent_shot,
        talent_defense,
        traits: current,
    } = pal;

    let id = instance_id.clone();
    let trait_id = instance_id;
    let stars = "\u{2605}".repeat(usize::from(stars));
    let title = nickname.unwrap_or(species);

    let edit_for = move |id: &str, pending: RwSignal<HashMap<String, PalEdit>>| {
        pending.get_untracked().get(id).cloned().unwrap_or_else(|| PalEdit {
            instance_id: id.to_string(),
            level: None,
            talent_hp: None,
            talent_shot: None,
            talent_defense: None,
            traits: None,
        })
    };

    let num_input = move |label: &'static str,
                          value: i64,
                          min: i64,
                          max: i64,
                          apply: fn(&mut PalEdit, i64)| {
        let id = id.clone();
        view! {
            <label class="save-field">
                {format!("{label} ({min}-{max})")}
                <input
                    type="number" min=min max=max value=value
                    on:change=move |ev| {
                        let Ok(v) = event_target_value(&ev).parse::<i64>() else {
                            return;
                        };
                        let v = v.clamp(min, max);
                        set_input_value(&ev, v);
                        let id = id.clone();
                        let mut edit = edit_for(&id, pending);
                        apply(&mut edit, v);
                        pending.update(|m| { let _ = m.insert(id, edit); });
                    }
                />
            </label>
        }
    };

    view! {
        <div class="save-pal">
            <div class="save-pal-id">
                <span class="save-species">{title}</span>
                <span class="save-tags">
                    {gender}
                    {if is_lucky { " Lucky" } else { "" }}
                    {if is_alpha { " Alpha" } else { "" }}
                    " " {stars}
                </span>
            </div>
            <div class="save-pal-stats">
                {num_input("Level", i64::from(level), 1, MAX_LEVEL, |e, v| {
                    e.level = i32::try_from(v).ok();
                })}
                {num_input("HP IV", i64::from(talent_hp), 0, MAX_TALENT, |e, v| {
                    e.talent_hp = u8::try_from(v).ok();
                })}
                {num_input("Atk IV", i64::from(talent_shot), 0, MAX_TALENT, |e, v| {
                    e.talent_shot = u8::try_from(v).ok();
                })}
                {num_input("Def IV", i64::from(talent_defense), 0, MAX_TALENT, |e, v| {
                    e.talent_defense = u8::try_from(v).ok();
                })}
            </div>
            <select
                class="save-traits" multiple=true
                on:change=move |ev| {
                    let selected = selected_values(&ev);
                    if selected.len() > MAX_TRAITS {
                        return;
                    }
                    let id = trait_id.clone();
                    let mut edit = edit_for(&id, pending);
                    edit.traits = Some(selected);
                    pending.update(|m| { let _ = m.insert(id, edit); });
                }
            >
                {options.into_iter().map(|(value, label)| {
                    let selected = current.contains(&value);
                    view! {
                        <option value=value selected=selected>{label}</option>
                    }
                }).collect_view()}
            </select>
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn set_input_value(ev: &leptos::ev::Event, value: i64) {
    use wasm_bindgen::JsCast;
    if let Some(input) =
        ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        input.set_value(&value.to_string());
    }
}

#[cfg(not(feature = "hydrate"))]
const fn set_input_value(_ev: &leptos::ev::Event, _value: i64) {}

fn grant_summary(current: i32, pending: Option<i32>) -> String {
    let Some(target) = pending else { return String::new() };
    let delta = target - current;
    if delta == 0 {
        return String::new();
    }
    let status = delta * STATUS_POINTS_PER_LEVEL;
    let tech = delta * TECH_POINTS_PER_LEVEL;
    if delta > 0 {
        format!(
            "{current} \u{2192} {target}: +{status} status points, \
             +{tech} technology points. Ancient points are untouched."
        )
    } else {
        format!(
            "{current} \u{2192} {target}: {status} status points, \
             {tech} technology points (removed, floored at zero)."
        )
    }
}

#[cfg(feature = "hydrate")]
fn selected_values(ev: &leptos::ev::Event) -> Vec<String> {
    use wasm_bindgen::JsCast;
    let Some(select) =
        ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
    else {
        return Vec::new();
    };
    let options = select.selected_options();
    (0..options.length())
        .filter_map(|i| options.item(i))
        .filter_map(|el| el.dyn_into::<web_sys::HtmlOptionElement>().ok())
        .map(|opt| opt.value())
        .collect()
}

#[cfg(not(feature = "hydrate"))]
const fn selected_values(_ev: &leptos::ev::Event) -> Vec<String> {
    Vec::new()
}

#[cfg(any(feature = "hydrate", test))]
fn disposition_filename(raw: &str) -> Option<String> {
    let name = raw
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| {
            key.eq_ignore_ascii_case("filename")
                .then(|| value.trim().trim_matches('"'))
        })?;

    if name.is_empty() || name.contains(['/', '\\']) {
        return None;
    }
    Some(name.to_string())
}

#[cfg(feature = "hydrate")]
async fn download_export(edits: &SaveEdits) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let body = serde_json::to_string(edits).map_err(|e| e.to_string())?;
    let window = web_sys::window().ok_or("no window")?;

    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_body(&wasm_bindgen::JsValue::from_str(&body));
    let headers = web_sys::Headers::new().map_err(|_e| "headers")?;
    headers.set("content-type", "application/json").map_err(|_e| "content-type")?;
    init.set_headers(&headers);

    let resp: web_sys::Response = JsFuture::from(
        window.fetch_with_str_and_init("/admin/palworld/save/export", &init),
    )
    .await
    .map_err(|_e| "request failed")?
    .dyn_into()
    .map_err(|_e| "bad response")?;

    if !resp.ok() {
        return Err(format!("server returned {}", resp.status()));
    }

    let filename = resp
        .headers()
        .get("content-disposition")
        .ok()
        .flatten()
        .as_deref()
        .and_then(disposition_filename)
        .unwrap_or_else(|| FALLBACK_EXPORT_NAME.to_string());

    let blob: web_sys::Blob = JsFuture::from(resp.blob().map_err(|_e| "blob")?)
        .await
        .map_err(|_e| "blob")?
        .dyn_into()
        .map_err(|_e| "blob")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_e| "object url")?;

    let document = window.document().ok_or("no document")?;
    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|_e| "anchor")?
        .dyn_into()
        .map_err(|_e| "anchor")?;
    anchor.set_href(&url);
    anchor.set_download(&filename);
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(filename)
}

#[cfg(not(feature = "hydrate"))]
async fn download_export(_edits: &SaveEdits) -> Result<String, String> {
    std::future::ready(()).await;
    Ok(FALLBACK_EXPORT_NAME.to_string())
}

#[cfg(test)]
mod tests {
    use super::disposition_filename;

    #[test]
    fn reads_the_servers_modified_name() {
        assert_eq!(
            disposition_filename(
                "attachment; filename=\"Level_modified_20260729-142530Z.sav\"",
            )
            .as_deref(),
            Some("Level_modified_20260729-142530Z.sav"),
        );
    }

    #[test]
    fn rejects_paths_and_missing_names() {
        assert_eq!(disposition_filename("attachment"), None);
        assert_eq!(disposition_filename("attachment; filename=\"\""), None);
        assert_eq!(
            disposition_filename("attachment; filename=\"../Level.sav\""),
            None,
        );
    }
}
