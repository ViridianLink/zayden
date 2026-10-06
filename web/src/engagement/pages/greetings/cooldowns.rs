use topcoat::Result;
use topcoat::view::{View, component, view};

use super::{GreetingAction, PAGE};
use crate::components::icons::{Icon, icon};
use crate::components::settings::{save_button, save_feedback, setting_field};
use crate::engagement::CooldownView;
use crate::engagement::pages::action::form_action;

#[component]
pub(super) async fn cooldown_section(
    guild_id: &str,
    cooldowns: CooldownView,
    outcome: Option<std::result::Result<(), &str>>,
) -> Result<impl View> {
    let action = form_action(guild_id, PAGE, GreetingAction::SaveCooldowns);
    let user_label = cooldowns.user_label();
    let guild_label = cooldowns.guild_label();
    let user_secs = cooldowns.user_secs.to_string();
    let guild_secs = cooldowns.guild_secs.to_string();

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Gauge)
                "Cooldowns"
            </legend>
            <p class="page-lead">
                "The per-member cooldown stops one person spamming "
                <code>"/good"</code>
                "; the server-wide one stops a crowd doing it between them. Both are in seconds, and both must stay at or above the minimum for this server's plan."
            </p>
            if let Some(result) = outcome {
                save_feedback(outcome: result)
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                setting_field(
                    label: user_label.as_str(),
                    name: "user_cooldown",
                    value: user_secs.as_str()
                )
                setting_field(
                    label: guild_label.as_str(),
                    name: "guild_cooldown",
                    value: guild_secs.as_str()
                )
                save_button()
            </form>
            if let Some(pitch) = cooldowns.upgrade_pitch() {
                <p class="page-lead">
                    (pitch)
                    " "
                    <a href="/upgrade">"See plans"</a>
                    "."
                </p>
            }
        </fieldset>
    })
}
