use leptos::prelude::*;
use leptos::task::spawn_local;

use super::icons::{Icon, module_icon, module_tint};
use crate::dto::ModuleView;
use crate::server::modules::set_module_enabled;

#[component]
pub(crate) fn ModuleCard(module: ModuleView, guild_id: String) -> impl IntoView {
    let ModuleView { id, label, description, enabled, commands: _ } = module;
    let icon = module_icon(&id);
    let tint_style = format!("--tint: {}", module_tint(&id));

    let desired = RwSignal::new(enabled);
    let synced = RwSignal::new(enabled);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_click = move |_| {
        error.set(None);
        desired.update(|v| *v = !*v);

        if saving.get_untracked() {
            return;
        }
        saving.set(true);

        let guild = guild_id.clone();
        let module_id = id.clone();

        spawn_local(async move {
            loop {
                let (Some(target), Some(current)) =
                    (desired.try_get_untracked(), synced.try_get_untracked())
                else {
                    return;
                };

                if target == current {
                    break;
                }

                match set_module_enabled(guild.clone(), module_id.clone(), target)
                    .await
                {
                    Ok(()) => {
                        synced.try_set(target);
                    },
                    Err(e) => {
                        error.try_set(Some(e.to_string()));
                        desired.try_set(current);
                        break;
                    },
                }
            }

            saving.try_set(false);
        });
    };

    let toggle_cls = move || {
        if desired.get() { "toggle toggle-on" } else { "toggle" }
    };

    let status = move || {
        if error.with(Option::is_some) {
            ("module-status failed", "Not saved")
        } else if saving.get() {
            ("module-status saving", "Saving\u{2026}")
        } else if desired.get() {
            ("module-status on", "Enabled")
        } else {
            ("module-status", "Disabled")
        }
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
                    aria-pressed=move || desired.get().to_string()
                    on:click=on_click
                />
            </div>
            <div class="module-name">{label}</div>
            <p class="module-desc">{description}</p>
            {move || error.get().map(|e| view! {
                <p class="module-error">{e}</p>
            })}
            <div class="module-card-foot">
                <span class=move || status().0>{move || status().1}</span>
            </div>
        </div>
    }
}
