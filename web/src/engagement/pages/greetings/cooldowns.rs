use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};
use zayden_app::config::GreetingsSettingsRow;

use super::{COOLDOWNS, GreetingAction, PAGE};
use crate::components::plan_note::plan_note;
use crate::components::save_bar::save_bar;
use crate::engagement::CooldownView;
use crate::engagement::pages::action::form_action;
use crate::engagement::pages::fields::{Constraints, form_summary, text_row};
use crate::engagement::pages::state::PageState;

const MAX_SECS: i32 = GreetingsSettingsRow::MAX_COOLDOWN_SECS;

fn limits(floor: i32, plan: &str) -> String {
    format!(
        "In seconds: at least {floor} on the {plan} plan, at most {MAX_SECS}. Leave blank for the minimum."
    )
}

fn bounds(floor: i32) -> Constraints {
    Constraints { min: Some(floor), max: Some(MAX_SECS), ..Constraints::default() }
}

#[component]
pub(super) async fn cooldown_section(
    guild_id: &str,
    cooldowns: CooldownView,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(COOLDOWNS);
    let plan = cooldowns.tier.label();
    let user_help = limits(cooldowns.floor_user_secs, plan);
    let guild_help = limits(cooldowns.floor_guild_secs, plan);
    let user_secs = cooldowns.user_secs.to_string();
    let guild_secs = cooldowns.guild_secs.to_string();

    Ok(view! {
        <section
            class="settings-section"
            id=(COOLDOWNS)
            aria-labelledby="greeting-cooldowns-title"
        >
            <h2 class="label" id="greeting-cooldowns-title">"Cooldowns"</h2>
            <p class="page-lead">
                "The per-member cooldown stops one person spamming "
                <code>"/good"</code>
                "; the server-wide one stops a crowd doing it between them. Both must stay at or above the minimum for this server's plan."
            </p>
            <form
                method="post"
                action=(form_action(guild_id, PAGE, GreetingAction::SaveCooldowns))
                data-pending=""
                data-dirty-guard=""
            >
                if let Some(message) = sent.summary() {
                    form_summary(form: COOLDOWNS, message: message)
                }
                <input type="hidden" name="guild" value=(guild_id)>
                text_row(
                    form: COOLDOWNS,
                    name: "user_cooldown",
                    label: "Per-member cooldown",
                    value: sent.value("user_cooldown", &user_secs),
                    help: Some(&user_help),
                    error: sent.error("user_cooldown"),
                    constraints: bounds(cooldowns.floor_user_secs)
                )
                text_row(
                    form: COOLDOWNS,
                    name: "guild_cooldown",
                    label: "Server-wide cooldown",
                    value: sent.value("guild_cooldown", &guild_secs),
                    help: Some(&guild_help),
                    error: sent.error("guild_cooldown"),
                    constraints: bounds(cooldowns.floor_guild_secs)
                )
                if let (Some(next), Some(pitch)) = (cooldowns.next_tier, cooldowns.upgrade_pitch()) {
                    plan_note(tier: next.label(), text: &pitch)
                }
                save_bar(notice: state.notice_for(COOLDOWNS))
            </form>
        </section>
    }
    .boxed())
}
