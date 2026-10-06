use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::{StatusCode, page, path_param};
use topcoat::view::{View, component, suspense, view};

use super::card::module_card;
use super::guild_layout::GuildId;
use crate::components::shape_skeleton::{SkeletonShape, shape_skeleton};
use crate::guild::GuildError;
use crate::guild::modules::{
    ModuleToggleForm,
    list_guild_modules,
    set_module_enabled,
};
use crate::util::server_error_text;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToggleFailure {
    module_id: Option<String>,
    message: String,
}

#[page("/guild/{guild_id}")]
pub(super) async fn guild_overview(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { overview(guild_id: guild_id) })
}

/// Applies a module toggle, then sends the browser back to the overview. A
/// toggle that fails re-renders the overview with the message on its card.
#[page(POST "/guild/{guild_id}")]
pub(super) async fn toggle_module(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let module_id = pairs
        .iter()
        .find(|(name, _)| name == "module_id")
        .map(|(_, value)| value.clone());

    let Err(error) = toggle(cx, guild_id, pairs).await else {
        return Err(see_other(format!("/guild/{guild_id}")).into());
    };
    let failure = ToggleFailure {
        module_id,
        message: server_error_text(error.redirect_unauthenticated()?),
    };

    Ok(view! {
        (StatusCode::UNPROCESSABLE_ENTITY)
        overview(guild_id: guild_id, failure: Some(failure))
    })
}

async fn toggle(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = ModuleToggleForm::from_pairs(pairs)?;
    if form.guild != guild_id {
        return Err(GuildError::InvalidField("guild"));
    }
    let enabled = form.enabled()?;

    set_module_enabled(cx, guild_id, &form.module_id, enabled).await
}

#[component]
async fn overview(
    guild_id: &str,
    #[default] failure: Option<ToggleFailure>,
) -> Result<impl View> {
    Ok(view! {
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>"Modules"</h1>
                    <p class="page-lead">
                        "Turn modules on or off for this server. A module that's off has its commands removed from the server."
                    </p>
                </div>
                <a
                    href=(format!("/guild/{guild_id}/settings/general"))
                    class="btn btn-secondary"
                >
                    "Server settings"
                </a>
            </div>
            suspense(
                fallback: view! {
                    <div class="skeleton-grid">
                        shape_skeleton(shape: SkeletonShape::Panel, count: 6)
                    </div>
                },
                module_grid(guild_id: guild_id, failure: failure)
            )
        </div>
    })
}

#[component]
async fn module_grid(
    cx: &Cx,
    guild_id: &str,
    failure: Option<ToggleFailure>,
) -> Result<impl View> {
    let modules = list_guild_modules(cx, guild_id).await.map_err(server_error_text);
    let unattached = match (&modules, &failure) {
        (Ok(modules), Some(failure))
            if !modules.iter().any(|module| {
                failure.module_id.as_deref() == Some(module.id.as_str())
            }) =>
        {
            Some(failure.message.clone())
        },
        _ => None,
    };

    Ok(view! {
        match modules {
            Err(error) => <p class="error">
                "Failed to load modules: "
                (error)
            </p>,
            Ok(modules) => {
                if let Some(message) = unattached {
                    <p class="error">(message)</p>
                }
                <div class="module-grid">
                    #[key(module.id.as_str())]
                    for module in &modules {
                        let error = failure
                            .as_ref()
                            .filter(
                                |failure| failure.module_id.as_deref()
                                        == Some(module.id.as_str()),
                            )
                            .map(|failure| failure.message.as_str());
                        module_card(module: module, guild_id: guild_id, error: error)
                    }
                </div>
            }
        }
    })
}
