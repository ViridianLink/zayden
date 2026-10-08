use serde::Serialize;
use topcoat::Result;
use topcoat::asset::Asset;
use topcoat::context::Cx;
use topcoat::router::{StatusCode, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::index::{draft_key, editor_id};
use super::view::editor;
use super::{LOADOUT_EDITOR_JS, LoadoutId};
use crate::admin::dto::{LoadoutCatalog, LoadoutForm};
use crate::admin::error::AdminError;
use crate::admin::loadouts::{get_loadout, loadout_catalog};
use crate::admin::pages::gate::{Gate, closed, gate};
use crate::auth::WebRole;
use crate::components::error_panel::{ErrorAction, error_panel};
use crate::shell::app_shell;

const LIST_PATH: &str = "/admin/destiny2/loadouts";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditorData<'a> {
    form: &'a LoadoutForm,
    catalog: &'a LoadoutCatalog,
    draft_key: String,
}

pub fn editor_data(catalog: &LoadoutCatalog, form: &LoadoutForm) -> Result<String> {
    let data = EditorData { form, catalog, draft_key: draft_key(form.id) };
    Ok(serde_json::to_string(&data)?.replace('<', "\\u003c"))
}

async fn load(
    cx: &Cx,
    id: Option<i32>,
) -> std::result::Result<(LoadoutCatalog, LoadoutForm), AdminError> {
    let catalog = loadout_catalog(cx).await?;
    let form = match id {
        Some(id) => get_loadout(cx, id).await?,
        None => catalog.blank.clone(),
    };
    Ok((catalog, form))
}

#[page("/admin/destiny2/loadouts/new")]
pub(super) async fn new_loadout(cx: &Cx) -> Result<impl View> {
    let access = gate(cx, WebRole::Admin).await?;
    if access != Gate::Open {
        return Ok(view! { closed(role: WebRole::Admin, gate: access) }.boxed());
    }

    Ok(view! { editor_page(id: None) }.boxed())
}

/// The editor for one loadout. An id that is not a whole number, or names no
/// loadout, is a 404 with a way back to the list.
#[page("/admin/destiny2/loadouts/{loadout_id}")]
pub(super) async fn edit_loadout(cx: &Cx) -> Result<impl View> {
    let access = gate(cx, WebRole::Admin).await?;
    if access != Gate::Open {
        return Ok(view! { closed(role: WebRole::Admin, gate: access) }.boxed());
    }
    let raw: &str = path_param::<LoadoutId>(cx);

    Ok(match editor_id(raw) {
        Some(id) => view! { editor_page(id: Some(id)) }.boxed(),
        None => view! { missing_loadout() }.boxed(),
    })
}

enum Loaded {
    Missing,
    Denied,
    Failed(String),
    Ready(Box<Editor>),
}

struct Editor {
    catalog: LoadoutCatalog,
    form: LoadoutForm,
    data: String,
}

#[component]
async fn editor_page(cx: &Cx, id: Option<i32>) -> Result<impl View> {
    let loaded = match load(cx, id).await {
        Err(AdminError::LoadoutNotFound(_)) => Loaded::Missing,
        Err(e) if e.is_denied() => Loaded::Denied,
        Err(e) => Loaded::Failed(e.to_string()),
        Ok((catalog, form)) => {
            let data = editor_data(&catalog, &form)?;
            Loaded::Ready(Box::new(Editor { catalog, form, data }))
        },
    };
    let script: Asset = LOADOUT_EDITOR_JS;
    let retry = id.map_or_else(
        || format!("{LIST_PATH}/new"),
        |id| format!("{LIST_PATH}/{id}"),
    );

    Ok(match loaded {
        Loaded::Missing => view! { missing_loadout() }.boxed(),
        Loaded::Denied => {
            view! { closed(role: WebRole::Admin, gate: Gate::Forbidden) }.boxed()
        },
        Loaded::Failed(reason) => {
            let message = format!("Something went wrong: {reason}");
            let actions = [
                ErrorAction::new("Try again", &retry),
                ErrorAction::new("Back to loadouts", LIST_PATH),
            ];
            view! {
                app_shell(
                    <div class="page">
                        error_panel(
                            title: "Couldn't load the loadout",
                            message: &message,
                            actions: &actions
                        )
                    </div>
                )
            }
            .boxed()
        },
        Loaded::Ready(ready) => view! {
            app_shell(
                <div class="page">
                    editor(
                        catalog: &ready.catalog,
                        form: &ready.form,
                        data: &ready.data,
                        script: script
                    )
                </div>
            )
        }
        .boxed(),
    })
}

#[component]
async fn missing_loadout() -> Result<impl View> {
    let back = [ErrorAction::new("Back to loadouts", LIST_PATH)];

    Ok(view! {
        (StatusCode::NOT_FOUND)
        app_shell(
            <div class="page">
                error_panel(
                    title: "Loadout not found",
                    message: "There is no loadout at this address. It may have been deleted, or the link has a typo.",
                    actions: &back
                )
            </div>
        )
    }
    .boxed())
}
