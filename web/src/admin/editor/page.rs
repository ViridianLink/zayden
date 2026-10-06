use serde::Serialize;
use topcoat::Result;
use topcoat::asset::Asset;
use topcoat::context::Cx;
use topcoat::router::{page, path_param};
use topcoat::view::{View, component, view};

use super::index::{draft_key, editor_id};
use super::view::editor;
use super::{LOADOUT_EDITOR_JS, LoadoutId};
use crate::admin::dto::{LoadoutCatalog, LoadoutForm};
use crate::admin::error::AdminError;
use crate::admin::loadouts::{get_loadout, loadout_catalog};
use crate::shell::app_shell;
use crate::util::server_error_text;

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
) -> Result<(LoadoutCatalog, LoadoutForm), AdminError> {
    let catalog = loadout_catalog(cx).await?;
    let form = match id {
        Some(id) => get_loadout(cx, id).await?,
        None => catalog.blank.clone(),
    };
    Ok((catalog, form))
}

#[page("/admin/destiny2/loadouts/new")]
pub(super) async fn new_loadout() -> Result<impl View> {
    Ok(view! { editor_page(id: None) })
}

#[page("/admin/destiny2/loadouts/{loadout_id}")]
pub(super) async fn edit_loadout(cx: &Cx) -> Result<impl View> {
    let raw: &str = path_param::<LoadoutId>(cx);
    let id = editor_id(raw);
    Ok(view! { editor_page(id: id) })
}

enum Loaded {
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
        Err(e) if e.is_denied() => Loaded::Denied,
        Err(e) => Loaded::Failed(server_error_text(e)),
        Ok((catalog, form)) => {
            let data = editor_data(&catalog, &form)?;
            Loaded::Ready(Box::new(Editor { catalog, form, data }))
        },
    };
    let script: Asset = LOADOUT_EDITOR_JS;

    Ok(view! {
        app_shell(
            <div class="page">
                match &loaded {
                    Loaded::Ready(ready) => editor(
                        catalog: &ready.catalog,
                        form: &ready.form,
                        data: &ready.data,
                        script: script
                    ),
                    Loaded::Denied => <p class="error">
                        "Admin access is required to edit loadouts."
                    </p>,
                    Loaded::Failed(error) => <p class="error">
                        "Couldn't load the loadout: "
                        (error.as_str())
                    </p>,
                }
            </div>
        )
    })
}
