use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use super::icons::{Icon, module_icon, module_tint};
use crate::dto::ModuleView;
use crate::server::modules::set_module_enabled;
use crate::ui::nav;

#[component]
pub(crate) fn ModuleCard(module: ModuleView, guild_id: String) -> impl IntoView {
    let ModuleView { id, label, description, enabled, locked, commands: _ } = module;
    let locked_reason = locked;
    let locked = locked_reason.is_some();
    let icon = module_icon(&id);
    let tint_style = format!("--tint: {}", module_tint(&id));

    let configure = nav::for_module(&id).map(|entry| {
        let href = entry.href(&guild_id);
        view! {
            <A href=href attr:class="module-configure">
                "Configure"
                <Icon name="chevron-right"/>
            </A>
        }
    });

    let unknown = enabled.is_none();
    let known = enabled.unwrap_or(false);

    let desired = RwSignal::new(known);
    let synced = RwSignal::new(known);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_click = move |_| {
        if locked || unknown {
            return;
        }

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
        if unknown {
            ("module-status", "Unknown")
        } else if error.with(Option::is_some) {
            ("module-status failed", "Not saved")
        } else if saving.get() {
            ("module-status saving", "Saving\u{2026}")
        } else if desired.get() {
            ("module-status on", "Enabled")
        } else {
            ("module-status", "Disabled")
        }
    };

    let unknown_note = unknown.then(|| {
        view! {
            <p class="module-locked">
                "Zayden couldn't read this module's current state, so it can't \
                 be changed right now."
            </p>
        }
    });

    let lock_note =
        locked_reason.map(|reason| view! { <p class="module-locked">{reason}</p> });

    view! {
        <div class="module-card">
            <div class="module-card-head">
                <div class="module-icon" style=tint_style>
                    <Icon name=icon/>
                </div>
                <button
                    class=toggle_cls
                    aria-label="Toggle module"
                    aria-pressed=move || {
                        if unknown {
                            "mixed".to_string()
                        } else {
                            desired.get().to_string()
                        }
                    }
                    disabled=locked || unknown
                    on:click=on_click
                />
            </div>
            <div class="module-name">{label}</div>
            <p class="module-desc">{description}</p>
            {unknown_note}
            {lock_note}
            {move || error.get().map(|e| view! {
                <p class="module-error">{e}</p>
            })}
            <div class="module-card-foot">
                <span class=move || status().0>{move || status().1}</span>
                {configure}
            </div>
        </div>
    }
}
