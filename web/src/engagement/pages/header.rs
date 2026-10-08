use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{Child, View, ViewExt, component, view};
use zayden_app::modules::{self, Backing};

use super::state::{Done, Failure, settle};
use crate::components::lamp::{LampState, lamp};
use crate::components::module_row::NOT_SYNCED_NOTE;
use crate::engagement::EngagementError;
use crate::form::fold;
use crate::guild::GuildError;
use crate::guild::modules::{list_guild_modules, set_module_enabled};

pub(super) const MODULE_FORM: &str = "module";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModuleState {
    Switch(bool),
    Status(bool),
    NotSynced,
}

const fn lamp_state(on: bool) -> LampState {
    if on { LampState::On } else { LampState::Off }
}

async fn module_state(
    cx: &Cx,
    guild_id: &str,
    module_id: &str,
) -> Option<ModuleState> {
    let backing = modules::find(module_id)?.backing;
    let views = list_guild_modules(cx, guild_id).await.ok()?;
    let enabled = views.into_iter().find(|view| view.id == module_id)?.enabled;

    Some(match (backing, enabled) {
        (_, None) => ModuleState::NotSynced,
        (Backing::Commands, Some(on)) => ModuleState::Switch(on),
        (Backing::Settings | Backing::Derived, Some(on)) => ModuleState::Status(on),
    })
}

#[component]
pub(super) async fn page_header(
    cx: &Cx,
    guild_id: &str,
    title: &str,
    #[default] module_id: Option<&str>,
    #[default] switch: Option<&str>,
    #[default] failure: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let module = match module_id {
        Some(module_id) => module_state(cx, guild_id, module_id).await,
        None => None,
    };
    let switch_label = format!("{title} module");

    Ok(view! {
        <div class="page-header">
            <div>
                <h1>(title)</h1>
                <p class="page-lead">(child)</p>
                if module == Some(ModuleState::NotSynced) {
                    <p class="field-hint">(NOT_SYNCED_NOTE)</p>
                }
                if let Some(failure) = failure {
                    <p class="error" role="alert">
                        "Not changed: "
                        (failure)
                    </p>
                }
            </div>
            match (module, switch) {
                (Some(ModuleState::Switch(on)), Some(action)) => <form
                    class="settings-actions"
                    method="post"
                    action=(action)
                    data-pending=""
                >
                    <input type="hidden" name="guild" value=(guild_id)>
                    lamp(state: lamp_state(on), pending: Some("Saving\u{2026}"))
                    <button
                        type="submit"
                        class="switch"
                        role="switch"
                        aria-checked=(if on { "true" } else { "false" })
                        aria-label=(switch_label.as_str())
                        name="enabled"
                        value=(if on { "false" } else { "true" })
                    >
                        <span class="switch-thumb" aria-hidden="true"></span>
                    </button>
                </form>,
                (Some(ModuleState::Switch(on) | ModuleState::Status(on)), _) => <div class="settings-actions">
                    lamp(state: lamp_state(on))
                </div>,
                (Some(ModuleState::NotSynced), _) => <div class="settings-actions">
                    lamp(state: LampState::NotSynced)
                </div>,
                (None, _) => "",
            }
        </div>
    }
    .boxed())
}

fn requested(
    pairs: Vec<(String, String)>,
    guild_id: &str,
) -> std::result::Result<bool, EngagementError> {
    let [guild, enabled] = fold(pairs, ["guild", "enabled"])?;
    if guild != guild_id {
        return Err(EngagementError::GuildMismatch);
    }
    match enabled.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(EngagementError::Guild(GuildError::InvalidField("enabled"))),
    }
}

pub(super) async fn switch_module(
    cx: &Cx,
    guild_id: &str,
    module_id: &str,
    label: &str,
    page: String,
    pairs: Vec<(String, String)>,
) -> Result<Failure> {
    let values = pairs.clone();
    let (result, enabled) = match requested(pairs, guild_id) {
        Ok(enabled) => (
            set_module_enabled(cx, guild_id, module_id, enabled)
                .await
                .map_err(EngagementError::from),
            enabled,
        ),
        Err(error) => (Err(error), false),
    };
    let message = format!("{label} turned {}.", if enabled { "on" } else { "off" });

    settle(cx, MODULE_FORM, values, result, &Done {
        page,
        section: None,
        message: &message,
    })
}
