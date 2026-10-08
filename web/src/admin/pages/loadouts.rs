#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::{StatusCode, page};
use topcoat::runtime::{Event, Signal, expr, signal};
use topcoat::view::{View, ViewExt, component, suspense, view};

use super::error::DeleteError;
use super::gate::{Gate, closed, gate};
use crate::admin::{
    LoadoutFieldError,
    LoadoutSummary,
    delete_loadout,
    list_loadouts,
};
use crate::auth::WebRole;
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::data_table::{data_cell, data_table};
use crate::components::flash::flash;
use crate::components::icons::{Icon, icon};
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::flash::{Flash, FlashKind, set as set_flash, take as take_flash};
use crate::form::fold;
use crate::shell::app_shell;
use crate::util::server_error_text;

const LIST_PATH: &str = "/admin/destiny2/loadouts";
const NEW_PATH: &str = "/admin/destiny2/loadouts/new";
const DELETED: &str = "Loadout deleted.";
const DELETE_PROMPT: &str = "This removes the build from /destiny2 builds for everyone. It cannot be undone.";
const CLASSES: [&str; 3] = ["Hunter", "Titan", "Warlock"];
const SEPARATOR: &str = "\n";

#[page("/admin/destiny2/loadouts")]
pub(super) async fn loadouts_page(cx: &Cx) -> Result<impl View> {
    let access = gate(cx, WebRole::Admin).await?;
    if access != Gate::Open {
        return Ok(view! { closed(role: WebRole::Admin, gate: access) }.boxed());
    }
    let notice = take_flash(cx);

    Ok(view! { app_shell(loadout_list_page(notice: notice.as_ref())) }.boxed())
}

#[page(POST "/admin/destiny2/loadouts")]
pub(super) async fn delete(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let access = gate(cx, WebRole::Admin).await?;
    if access != Gate::Open {
        return Ok(view! { closed(role: WebRole::Admin, gate: access) }.boxed());
    }
    let Err(error) = remove(cx, pairs).await else {
        set_flash(cx, FlashKind::Success, DELETED)?;
        return Err(see_other(LIST_PATH).into());
    };
    let failure = error.to_string();

    Ok(view! {
        (StatusCode::UNPROCESSABLE_ENTITY)
        app_shell(loadout_list_page(failure: Some(failure.as_str())))
    }
    .boxed())
}

async fn remove(
    cx: &Cx,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), DeleteError> {
    let [id] = fold(pairs, ["id"])?;
    let Ok(id) = id.parse::<i32>() else {
        return Err(LoadoutFieldError::InvalidId(id).into());
    };

    Ok(delete_loadout(cx, id).await?)
}

#[component]
pub async fn loadout_list_page(
    cx: &Cx,
    #[default] notice: Option<&Flash>,
    #[default] failure: Option<&str>,
) -> Result<impl View> {
    let class = signal(cx, String::new);

    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Loadout builder"</h1>
                    <p class="page-lead">
                        "Builds shown by /destiny2 builds. Saves apply to the bot immediately."
                    </p>
                </div>
                <a href=(NEW_PATH) class="btn btn-primary">"New loadout"</a>
            </div>
            flash(notice: notice)
            if let Some(message) = failure {
                <div
                    class="error"
                    id="delete-summary"
                    role="alert"
                    tabindex="-1"
                    autofocus=""
                >
                    (format!("Not deleted: {message}"))
                </div>
            }
            <div class="field-row">
                <label class="field-label" for="loadout-class-filter">"Class"</label>
                <div class="select">
                    <select
                        class="input loadout-filter"
                        id="loadout-class-filter"
                        @change=$(|e: Event| class.set(e.target.value))
                    >
                        <option value="">"All classes"</option>
                        #[key(*option)]
                        for option in CLASSES {
                            <option value=(option)>(option)</option>
                        }
                    </select>
                    <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
                </div>
            </div>
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        shape_skeleton(shape: SkeletonShape::Row, count: 6)
                    </div>
                },
                loadout_table(class: &class)
            )
        </div>
    }
    .boxed())
}

fn shown_for(classes: &str, chosen: &str) -> f64 {
    if classes.is_empty() {
        return 0.0;
    }
    let count = classes
        .split(SEPARATOR)
        .filter(|class| chosen.is_empty() || *class == chosen)
        .count();
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

#[component]
async fn loadout_table(cx: &Cx, class: &Signal<String>) -> Result<impl View> {
    let rows = list_loadouts(cx)
        .await
        .map_err(|e| if e.is_denied() { None } else { Some(server_error_text(e)) });

    Ok(view! {
        match rows {
            Err(None) => <p class="error">
                "Admin access is required to edit loadouts."
            </p>,
            Err(Some(text)) => <p class="error">
                "Couldn't load loadouts: "
                (text)
            </p>,
            Ok(rows) => loadout_rows(rows: &rows, class: class),
        }
    }
    .boxed())
}

#[component]
async fn loadout_rows(
    rows: &[LoadoutSummary],
    class: &Signal<String>,
) -> Result<impl View> {
    let total = rows.len();
    let classes =
        rows.iter().map(|row| row.class.clone()).collect::<Vec<_>>().join(SEPARATOR);
    let shown = expr!({
        let chosen = class.get();
        let all = classes;
        raw!(
            "cx.hydrate(${all}.toString() === '' ? 0 : ${all}.toString().split(String.fromCharCode(10)).filter((c) => ${chosen}.toString() === '' || c === ${chosen}.toString()).length)",
            shown_for(&all, &chosen)
        )
    });

    Ok(view! {
        <p class="operator-count" role="status">
            $(shown)
            " of "
            (total)
            " loadouts"
        </p>
        if rows.is_empty() {
            <p class="page-lead">"No loadouts yet - start one with New loadout."</p>
        } else {
            data_table(
                caption: "Loadouts",
                columns: &["Loadout", "Class", "Subclass", "Mode", "Author", "Action"],
                #[key(loadout.id)]
                for loadout in rows {
                    loadout_row(loadout: loadout, class: class)
                }
            )
        }
    }
    .boxed())
}

#[component]
async fn loadout_row(
    loadout: &LoadoutSummary,
    class: &Signal<String>,
) -> Result<impl View> {
    let href = format!("{LIST_PATH}/{}", loadout.id);
    let own_class = loadout.class.clone();
    let delete_id = format!("loadout-{}-delete", loadout.id);
    let delete_title = format!("Delete loadout \u{201c}{}\u{201d}?", loadout.name);
    let hidden =
        expr!(if class.get().is_empty() { false } else { class.get() != own_class });

    Ok(view! {
        <tr role="row" :hidden=$(hidden)>
            data_cell(
                label: "Loadout",
                header: true,
                <a href=(href.as_str()) class="loadout-name">(loadout.name.as_str())</a>
            )
            data_cell(label: "Class", (loadout.class.as_str()))
            data_cell(label: "Subclass", (loadout.element.as_str()))
            data_cell(label: "Mode", (loadout.mode.as_str()))
            data_cell(label: "Author", (loadout.author.as_str()))
            data_cell(
                label: "Action",
                <form method="post" action=(LIST_PATH) data-pending="">
                    <input type="hidden" name="id" value=(loadout.id.to_string())>
                    confirm_dialog(
                        id: &delete_id,
                        trigger: "Delete",
                        title: &delete_title,
                        description: Some(DELETE_PROMPT),
                        confirm: "Delete loadout"
                    )
                </form>
            )
        </tr>
    }
    .boxed())
}
