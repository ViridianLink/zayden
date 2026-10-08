#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};

use super::{ADD_CHANNEL, CHANNELS, GreetingAction, PAGE, REMOVE_CHANNEL};
use crate::auth::ChannelInfo;
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::data_table::{data_cell, data_row, data_table};
use crate::components::flash::flash;
use crate::components::pickers::{Channel, channel_options};
use crate::engagement::pages::action::form_action;
use crate::engagement::pages::fields::{form_summary, select_row};
use crate::engagement::{GATE_KINDS, channel_label, unconfigured_channels};
use crate::flash::Flash;

#[component]
pub async fn channel_section(
    guild_id: &str,
    allowed: Option<&[String]>,
    channels: &[ChannelInfo],
    locked: bool,
    #[default] notice: Option<&Flash>,
    #[default] add_error: Option<&str>,
    #[default] add_field_error: Option<&str>,
    #[default] remove_error: Option<&str>,
    #[default] chosen: &str,
) -> Result<impl View> {
    let unknown = allowed.is_none();
    let allowed = allowed.unwrap_or_default();
    let unconfigured: Vec<Channel> =
        unconfigured_channels(channels, allowed).iter().map(Channel::from).collect();
    let choices =
        channel_options(Ok(unconfigured.as_slice()), GATE_KINDS).unwrap_or_default();
    let editable = !unknown && !locked;
    let columns: &[&str] =
        if editable { &["Channel", "Action"] } else { &["Channel"] };

    Ok(view! {
        <section
            class="settings-section"
            id=(CHANNELS)
            aria-labelledby="good-channels-title"
        >
            <h2 class="label" id="good-channels-title">"Where /good works"</h2>
            if !unknown {
                <p class="page-lead">
                    "With nothing listed, "
                    <code>"/good"</code>
                    " works in every channel. Add one or more and Discord hides the command everywhere else - it never even shows up in the picker. Adding a category covers every channel inside it."
                </p>
            }
            <p class="page-lead">
                "This writes the same command permissions as Discord's own Server Settings \u{2192} Integrations panel, so changes made either way show up in both."
            </p>
            flash(notice: notice)
            if let Some(message) = remove_error {
                form_summary(form: REMOVE_CHANNEL, message: message, outcome: "Not removed")
            }
            if unknown {
                if let Some(message) = add_error {
                    form_summary(form: ADD_CHANNEL, message: message, outcome: "Not added")
                }
                <p class="warning">
                    "Discord didn't report which channels "
                    <code>"/good"</code>
                    " is allowed in, so its restrictions can't be shown or changed right now."
                </p>
            } else {
                if allowed.is_empty() {
                    <p class="page-lead">
                        "No channels listed: "
                        <code>"/good"</code>
                        " works everywhere."
                    </p>
                } else {
                    data_table(
                        caption: "Channels where /good works",
                        columns: columns,
                        #[key(id.as_str())]
                        for (index, id) in allowed.iter().enumerate() {
                            channel_row(
                                guild_id: guild_id,
                                index: index,
                                label: channel_label(channels, id),
                                id: id,
                                editable: editable
                            )
                        }
                    )
                }
                if locked {
                    if let Some(message) = add_error {
                        form_summary(form: ADD_CHANNEL, message: message, outcome: "Not added")
                    }
                    <p class="warning">
                        "Read-only: Discord only lets a member with Manage Server change which channels a command is allowed in."
                    </p>
                } else {
                    <form
                        method="post"
                        action=(form_action(guild_id, PAGE, GreetingAction::AddChannel))
                        data-pending=""
                        data-dirty-guard=""
                    >
                        if let Some(message) = add_error {
                            form_summary(form: ADD_CHANNEL, message: message, outcome: "Not added")
                        }
                        <input type="hidden" name="guild" value=(guild_id)>
                        select_row(
                            form: ADD_CHANNEL,
                            name: "channel_id",
                            label: "Allow a channel",
                            selected: chosen,
                            options: &choices,
                            help: Some("A text, announcement or forum channel, or a whole category."),
                            error: add_field_error,
                            required: true
                        )
                        <div class="form-actions">
                            <button
                                type="submit"
                                class="btn btn-secondary"
                                data-pending-label="Adding\u{2026}"
                            >
                                "Add channel"
                            </button>
                        </div>
                    </form>
                }
            }
        </section>
    }
    .boxed())
}

#[component]
async fn channel_row(
    guild_id: &str,
    index: usize,
    label: String,
    id: &str,
    editable: bool,
) -> Result<impl View> {
    let action = form_action(guild_id, PAGE, GreetingAction::RemoveChannel);
    let confirm_id = format!("good-channel-{index}-remove");
    let confirm_title = format!("Remove {label} from the /good list?");

    Ok(view! {
        data_row(
            data_cell(label: "Channel", header: true, (label.as_str()))
            if editable {
                data_cell(
                    label: "Action",
                    <form method="post" action=(action) data-pending="">
                        <input type="hidden" name="guild" value=(guild_id)>
                        <input type="hidden" name="channel_id" value=(id)>
                        confirm_dialog(
                            id: &confirm_id,
                            trigger: "Remove",
                            title: &confirm_title,
                            description: Some(
                                "Removing the last channel lets /good work everywhere again.",
                            ),
                            confirm: "Remove channel"
                        )
                    </form>
                )
            }
        )
    })
}
