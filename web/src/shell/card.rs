#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};
use zayden_app::modules::{self, Backing};

use crate::components::module_row::{ModuleControl, module_row};
use crate::guild::dto::ModuleView;
use crate::nav;

/// What a module's rack row offers: a switch only for command-backed modules
/// whose state has synced; status and a link for modules switched elsewhere
/// (AI Chat from its form, Patreon and YouTube from their connection); the
/// not-synced note when no state exists yet.
fn module_control(module: &ModuleView) -> ModuleControl {
    let backing = modules::find(&module.id).map(|def| def.backing);

    match (module.enabled, backing) {
        (None, _) => ModuleControl::NotSynced,
        (Some(on), Some(Backing::Commands)) => ModuleControl::Switch { on },
        (Some(on), _) => ModuleControl::Status { on },
    }
}

/// One module in the overview rack. `error` is a failed switch's message;
/// the switch then still shows the stored state.
#[component]
pub async fn module_card(
    module: &ModuleView,
    guild_id: &str,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    let href = nav::for_module(&module.id).map(|entry| entry.href(guild_id));

    Ok(view! {
        module_row(
            guild_id: guild_id,
            module_id: &module.id,
            label: &module.label,
            description: &module.description,
            control: module_control(module),
            href: href.as_deref(),
            error: error
        )
    })
}
