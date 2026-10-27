use greetings::MAX_MESSAGE_LEN;
use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};

use super::{GreetingAction, MESSAGES, PAGE};
use crate::app::guild::guild_id::engagement::action::form_action;
use crate::app::guild::guild_id::engagement::state::PageState;
use crate::components::field_row::{form_summary, text_row};
use crate::components::placeholder_legend::placeholder_legend;
use crate::components::save_bar::save_bar;

const LEGEND: &[(&str, &str)] = &[
    (
        "{user}",
        "mentions the person being greeted, or the sender when the command is run without a user.",
    ),
    ("{author}", "mentions whoever ran the command."),
];

#[component]
pub(super) async fn messages_section(
    guild_id: &str,
    morning: &str,
    night: &str,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(MESSAGES);
    let help = format!(
        "Up to {MAX_MESSAGE_LEN} characters. Leave blank to post just the image."
    );

    Ok(view! {
        <section
            class="settings-section"
            id=(MESSAGES)
            aria-labelledby="greeting-messages-title"
        >
            <h2 class="label" id="greeting-messages-title">"Messages"</h2>
            <form
                method="post"
                action=(form_action(guild_id, PAGE, GreetingAction::SaveMessages))
                data-pending=""
                data-dirty-guard=""
            >
                if let Some(message) = sent.summary() {
                    form_summary(form: MESSAGES, message: message)
                }
                <input type="hidden" name="guild" value=(guild_id)>
                text_row(
                    form: MESSAGES,
                    name: "morning_message",
                    label: "Good morning message",
                    value: sent.value("morning_message", morning),
                    help: Some(&help),
                    error: sent.error("morning_message")
                )
                text_row(
                    form: MESSAGES,
                    name: "night_message",
                    label: "Good night message",
                    value: sent.value("night_message", night),
                    help: Some(&help),
                    error: sent.error("night_message")
                )
                placeholder_legend(entries: LEGEND)
                save_bar(notice: state.notice_for(MESSAGES))
            </form>
        </section>
    }
    .boxed())
}
