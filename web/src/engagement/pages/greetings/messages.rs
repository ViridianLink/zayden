use greetings::MAX_MESSAGE_LEN;
use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};

use super::{GreetingAction, MESSAGES, PAGE};
use crate::components::save_bar::save_bar;
use crate::engagement::pages::action::form_action;
use crate::engagement::pages::fields::{form_summary, text_row};
use crate::engagement::pages::state::PageState;

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
                placeholder_legend()
                save_bar(notice: state.notice_for(MESSAGES))
            </form>
        </section>
    }
    .boxed())
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
        </ul>
    })
}
