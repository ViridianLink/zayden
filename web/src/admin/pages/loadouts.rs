#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::request::uri;
use topcoat::router::{StatusCode, page};
use topcoat::runtime::{Event, Signal, expr, signal};
use topcoat::view::{View, component, suspense, view};
use url::form_urlencoded;

use super::error::DeleteError;
use crate::admin::{
    LoadoutFieldError,
    LoadoutSummary,
    delete_loadout,
    list_loadouts,
};
use crate::components::confirm::confirm_button;
use crate::components::settings::delete_feedback;
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::form::fold;
use crate::shell::app_shell;
use crate::shell::link::aria_current;
use crate::util::server_error_text;

const LIST_PATH: &str = "/admin/destiny2/loadouts";
const NEW_PATH: &str = "/admin/destiny2/loadouts/new";
const DELETED_PATH: &str = "/admin/destiny2/loadouts?deleted=1";
const DELETE_PROMPT: &str = "This removes the build from /destiny2 builds for everyone. It cannot be undone.";

type DeleteOutcome = std::result::Result<(), String>;

#[page("/admin/destiny2/loadouts")]
pub(super) async fn loadouts_page(cx: &Cx) -> Result<impl View> {
    let deleted =
        form_urlencoded::parse(uri(cx).query().unwrap_or_default().as_bytes())
            .any(|(key, value)| key == "deleted" && value == "1");

    Ok(view! { app_shell(loadout_list_page(deleted: deleted.then_some(Ok(())))) })
}

#[page(POST "/admin/destiny2/loadouts")]
pub(super) async fn delete(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    if let Err(e) = remove(cx, pairs).await {
        return Ok(view! {
            (StatusCode::UNPROCESSABLE_ENTITY)
            app_shell(loadout_list_page(deleted: Some(Err(e.to_string()))))
        });
    }

    Err(see_other(DELETED_PATH).into())
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
    #[default] deleted: Option<DeleteOutcome>,
) -> Result<impl View> {
    let class = signal(cx, String::new);
    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Destiny 2 Loadouts"</h1>
                    <p class="page-lead">
                        "Builds shown by /destiny2 builds. Saves apply to the bot immediately."
                    </p>
                </div>
                <a
                    href=(NEW_PATH)
                    aria-current=(aria_current(NEW_PATH, uri(cx).path()))
                    class="btn btn-primary"
                >
                    "New loadout"
                </a>
            </div>
            match deleted {
                None => {

                }
                Some(Ok(())) => delete_feedback(outcome: Ok(())),
                Some(Err(message)) => delete_feedback(outcome: Err(message.as_str())),
            }
            <select
                class="input loadout-filter"
                aria-label="Filter by class"
                @change=$(|e: Event| class.set(e.target.value))
            >
                <option value="">"All classes"</option>
                <option value="Hunter">"Hunter"</option>
                <option value="Titan">"Titan"</option>
                <option value="Warlock">"Warlock"</option>
            </select>
            suspense(
                fallback: view! {
                    <div class="skeleton-list">
                        shape_skeleton(shape: SkeletonShape::Row, count: 6)
                    </div>
                },
                loadout_table(class: &class)
            )
        </div>
    })
}

#[component]
async fn loadout_table(cx: &Cx, class: &Signal<String>) -> Result<impl View> {
    let rows = list_loadouts(cx)
        .await
        .map_err(|e| if e.is_denied() { None } else { Some(server_error_text(e)) });
    let location = uri(cx).path();

    Ok(view! {
        match rows {
            Err(None) => <p class="error">
                "Admin access is required to edit loadouts."
            </p>,
            Err(Some(text)) => <p class="error">
                "Couldn't load loadouts: "
                (text)
            </p>,
            Ok(rows) => {
                <div class="loadout-table">
                    #[key(loadout.id)]
                    for loadout in &rows {
                        loadout_row(loadout: loadout, class: class, location: location)
                    }
                </div>
            }
        }
    })
}

#[component]
async fn loadout_row(
    loadout: &LoadoutSummary,
    class: &Signal<String>,
    location: &str,
) -> Result<impl View> {
    let href = format!("/admin/destiny2/loadouts/{}", loadout.id);
    let meta = format!(
        "{} \u{2022} {} \u{2022} {} \u{2022} by {}",
        loadout.class, loadout.element, loadout.mode, loadout.author
    );
    let own_class = loadout.class.clone();
    let delete_id = format!("loadout-{}-delete", loadout.id);
    let delete_object = format!("\u{201c}{}\u{201d}", loadout.name);
    let hidden =
        expr!(if class.get().is_empty() { false } else { class.get() != own_class });

    Ok(view! {
        <div class="loadout-row" :hidden=$(hidden)>
            <a
                href=(href.as_str())
                aria-current=(aria_current(&href, location))
                class="loadout-name"
            >
                (loadout.name.as_str())
            </a>
            <span class="loadout-meta">(meta)</span>
            <form method="post" action=(LIST_PATH) data-pending="">
                <input type="hidden" name="id" value=(loadout.id.to_string())>
                confirm_button(
                    id: &delete_id,
                    label: "Delete",
                    prompt: DELETE_PROMPT,
                    confirm: "Delete loadout",
                    object: Some(&delete_object)
                )
            </form>
        </div>
    })
}
