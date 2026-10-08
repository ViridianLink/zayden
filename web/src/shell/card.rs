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

fn module_control(module: &ModuleView) -> ModuleControl {
    let backing = modules::find(&module.id).map(|def| def.backing);

    match (module.enabled, backing) {
        (None, _) => ModuleControl::NotSynced,
        (Some(on), Some(Backing::Commands)) => ModuleControl::Switch { on },
        (Some(on), _) => ModuleControl::Status { on },
    }
}

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
