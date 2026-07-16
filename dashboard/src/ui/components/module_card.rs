use leptos::prelude::*;

use super::icons::{Icon, module_icon, module_tint};
use crate::dto::ModuleView;
use crate::server::modules::SetModuleEnabled;

#[component]
pub(crate) fn ModuleCard(
    module: ModuleView,
    guild_id: String,
    toggle: ServerAction<SetModuleEnabled>,
) -> impl IntoView {
    let ModuleView { id, label, description, enabled, commands: _ } = module;
    let icon = module_icon(&id);
    let tint_style = format!("--tint: {}", module_tint(&id));

    let toggle_cls = if enabled { "toggle toggle-on" } else { "toggle" };
    let (status_cls, status_text) = if enabled {
        ("module-status on", "Enabled")
    } else {
        ("module-status", "Disabled")
    };

    view! {
        <div class="module-card">
            <div class="module-card-head">
                <div class="module-icon" style=tint_style>
                    <Icon name=icon/>
                </div>
                <button
                    class=toggle_cls
                    aria-label="Toggle module"
                    on:click=move |_| {
                        toggle.dispatch(SetModuleEnabled {
                            guild: guild_id.clone(),
                            module_id: id.clone(),
                            enabled: !enabled,
                        });
                    }
                />
            </div>
            <div class="module-name">{label}</div>
            <p class="module-desc">{description}</p>
            <div class="module-card-foot">
                <span class=status_cls>{status_text}</span>
            </div>
        </div>
    }
}
