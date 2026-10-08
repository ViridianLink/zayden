use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{View, ViewExt, component, view};
use zayden_app::modules::{self, Backing};

use super::Page;
use super::state::{Done, Failure, PageState, settle};
use crate::components::lamp::{LampState, lamp};
use crate::components::module_row::NOT_SYNCED_NOTE;
use crate::form::fold;
use crate::guild::GuildError;
use crate::guild::modules::{list_guild_modules, set_module_enabled};

pub(super) const MODULE_FORM: &str = "module";

pub(super) const MODULE_ACTION: &str = "module";

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
pub(super) async fn feature_header(
    cx: &Cx,
    guild_id: &str,
    page: Page,
    state: &PageState,
) -> Result<impl View> {
    let entry = page.entry();
    let module = match entry.module_id {
        Some(module_id) => module_state(cx, guild_id, module_id).await,
        None => None,
    };
    let error = state.sent(MODULE_FORM).summary();
    let action = format!("{}/{MODULE_ACTION}", entry.href(guild_id));
    let switch_label = format!("{} module", entry.label);

    Ok(view! {
        <div class="page-header">
            <div>
                <h1>(entry.label)</h1>
                <p class="page-lead">(entry.lead())</p>
                if module == Some(ModuleState::NotSynced) {
                    <p class="field-hint">(NOT_SYNCED_NOTE)</p>
                }
                if let Some(error) = error {
                    <p class="error" role="alert">
                        "Not changed: "
                        (error)
                    </p>
                }
            </div>
            match module {
                Some(ModuleState::Switch(on)) => <form
                    class="settings-actions"
                    method="post"
                    action=(action.as_str())
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
                Some(ModuleState::Status(on)) => <div class="settings-actions">
                    lamp(state: lamp_state(on))
                </div>,
                Some(ModuleState::NotSynced) => <div class="settings-actions">
                    lamp(state: LampState::NotSynced)
                </div>,
                None => "",
            }
        </div>
    }
    .boxed())
}

fn requested(
    pairs: Vec<(String, String)>,
    guild_id: &str,
) -> std::result::Result<bool, GuildError> {
    let [guild, enabled] = fold(pairs, ["guild", "enabled"])?;
    if guild != guild_id {
        return Err(GuildError::InvalidField("guild"));
    }
    match enabled.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(GuildError::InvalidField("enabled")),
    }
}

pub(super) async fn switch_module(
    cx: &Cx,
    guild_id: &str,
    page: Page,
    pairs: Vec<(String, String)>,
) -> Result<Failure> {
    let entry = page.entry();
    let values = pairs.clone();
    let (result, enabled) = match (entry.module_id, requested(pairs, guild_id)) {
        (Some(module_id), Ok(enabled)) => {
            (set_module_enabled(cx, guild_id, module_id, enabled).await, enabled)
        },
        (None, _) => (Err(GuildError::UnknownModule), false),
        (_, Err(error)) => (Err(error), false),
    };
    let message =
        format!("{} turned {}.", entry.label, if enabled { "on" } else { "off" });

    settle(cx, MODULE_FORM, values, result, &Done {
        page: page.href(guild_id),
        section: None,
        message: &message,
    })
}
