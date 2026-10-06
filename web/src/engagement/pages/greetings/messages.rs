use topcoat::Result;
use topcoat::view::{View, component, view};

use super::{GreetingAction, PAGE};
use crate::components::icons::{Icon, icon};
use crate::components::settings::{save_button, save_feedback, setting_field};
use crate::engagement::pages::action::form_action;

const ANY_TEXT: &str = ".*";

#[component]
pub(super) async fn messages_section(
    guild_id: &str,
    morning: &str,
    night: &str,
    outcome: Option<std::result::Result<(), &str>>,
) -> Result<impl View> {
    let action = form_action(guild_id, PAGE, GreetingAction::SaveMessages);

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Message)
                "Messages"
            </legend>
            if let Some(result) = outcome {
                save_feedback(outcome: result)
            }
            <form method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                setting_field(
                    label: "Good morning message",
                    name: "morning_message",
                    value: morning,
                    pattern: ANY_TEXT
                )
                setting_field(
                    label: "Good night message",
                    name: "night_message",
                    value: night,
                    pattern: ANY_TEXT
                )
                placeholder_legend()
                save_button()
            </form>
        </fieldset>
    })
}

#[component]
async fn placeholder_legend() -> Result<impl View> {
    Ok(view! {
        <ul class="greet-legend">
            <li>
                <code>"{user}"</code>
                " - mentions the person being greeted, or the sender when the command is run without a user."
            </li>
            <li>
                <code>"{author}"</code>
                " - mentions whoever ran the command."
            </li>
            <li>"Leave a message blank to post just the image."</li>
        </ul>
    })
}
