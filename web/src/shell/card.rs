#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use crate::components::icons::{Icon, icon, module_icon, module_tint};
use crate::guild::dto::ModuleView;
use crate::nav;

const UNKNOWN_NOTE: &str = "Zayden hasn't set this module up for this server yet, so it can't be changed right now.";

/// One module on the guild overview. A switchable module's toggle submits
/// the form at the end of the card, asking for the opposite of the stored
/// state. `error` is a failed toggle's message; the switch then shows the
/// stored state and the status reads "Not saved".
#[component]
pub async fn module_card(
    module: &ModuleView,
    guild_id: &str,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    let unknown = module.enabled.is_none();
    let on = module.enabled.unwrap_or(false);
    let switchable = !unknown && module.locked.is_none();
    let form_id = format!("module-toggle-{}", module.id);

    let (status_class, status) = if unknown {
        ("module-status", "Unknown")
    } else if error.is_some() {
        ("module-status failed", "Not saved")
    } else if on {
        ("module-status on", "Enabled")
    } else {
        ("module-status", "Disabled")
    };

    Ok(view! {
        <div class="module-card">
            <div class="module-card-head">
                <div
                    class="module-icon"
                    style=(format!("--tint: {}", module_tint(&module.id)))
                >
                    icon(name: module_icon(&module.id))
                </div>
                <button
                    class=(if on { "toggle toggle-on" } else { "toggle" })
                    aria-label="Toggle module"
                    aria-pressed=(if unknown {
                        "mixed"
                    } else if on {
                        "true"
                    } else {
                        "false"
                    })
                    disabled=(!switchable)
                    form=(switchable.then_some(form_id.as_str()))
                    name=(switchable.then_some("enabled"))
                    value=(switchable.then_some(if on { "false" } else { "true" }))
                ></button>
            </div>
            <div class="module-name">(module.label.as_str())</div>
            <p class="module-desc">(module.description.as_str())</p>
            if unknown {
                <p class="module-locked">(UNKNOWN_NOTE)</p>
            }
            if let Some(reason) = &module.locked {
                <p class="module-locked">(reason.as_str())</p>
            }
            if let Some(error) = error {
                <p class="module-error">(error)</p>
            }
            <div class="module-card-foot">
                <span class=(status_class)>(status)</span>
                if let Some(entry) = nav::for_module(&module.id) {
                    <a href=(entry.href(guild_id)) class="module-configure">
                        "Configure"
                        icon(name: Icon::ChevronRight)
                    </a>
                }
            </div>
            if switchable {
                <form
                    id=(form_id.as_str())
                    method="post"
                    action=(format!("/guild/{guild_id}"))
                >
                    <input type="hidden" name="guild" value=(guild_id)>
                    <input type="hidden" name="module_id" value=(module.id.as_str())>
                </form>
            }
        </div>
    })
}
