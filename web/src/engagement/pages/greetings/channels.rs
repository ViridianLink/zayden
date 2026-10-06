#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use super::{GreetingAction, PAGE};
use crate::auth::ChannelInfo;
use crate::components::icons::{Icon, icon};
use crate::components::select::{Channel, channel_select};
use crate::components::settings::save_feedback;
use crate::engagement::pages::action::form_action;
use crate::engagement::{GATE_KINDS, channel_label, unconfigured_channels};

#[component]
pub async fn channel_section(
    guild_id: &str,
    allowed: Option<&[String]>,
    channels: &[ChannelInfo],
    locked: bool,
    added: Option<std::result::Result<(), &str>>,
    removed: Option<std::result::Result<(), &str>>,
) -> Result<impl View> {
    let unknown = allowed.is_none();
    let allowed = allowed.unwrap_or_default();
    let unconfigured: Vec<Channel> =
        unconfigured_channels(channels, allowed).iter().map(Channel::from).collect();

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Grid)
                "Where /good works"
            </legend>
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
            <div class="chip-list">
                #[key(index)]
                for (index, id) in allowed.iter().enumerate() {
                    chip(
                        guild_id: guild_id,
                        label: channel_label(channels, id),
                        id: id,
                        locked: locked
                    )
                }
            </div>
            if unknown {
                <p class="module-locked">
                    "Discord didn't report which channels "
                    <code>"/good"</code>
                    " is allowed in, so its restrictions can't be shown or changed right now."
                </p>
            } else if locked {
                <p class="module-locked">
                    "Read-only: Discord only lets a member with Manage Server change which channels a command is allowed in."
                </p>
            } else {
                if let Some(outcome) = removed {
                    save_feedback(outcome: outcome)
                }
                if let Some(outcome) = added {
                    save_feedback(outcome: outcome)
                }
                <form
                    class="chip-add"
                    method="post"
                    action=(form_action(guild_id, PAGE, GreetingAction::AddChannel))
                    data-pending=""
                >
                    <input type="hidden" name="guild" value=(guild_id)>
                    channel_select(
                        label: "Allow a channel",
                        name: "channel_id",
                        selected: "",
                        channels: Ok(unconfigured.as_slice()),
                        kinds: GATE_KINDS
                    )
                    <button type="submit" class="btn btn-ghost">"Add channel"</button>
                </form>
            }
        </fieldset>
    })
}

#[component]
async fn chip(
    guild_id: &str,
    label: String,
    id: &str,
    locked: bool,
) -> Result<impl View> {
    let action = form_action(guild_id, PAGE, GreetingAction::RemoveChannel);

    Ok(view! {
        if locked {
            <span class="chip"><span class="chip-label">(label)</span></span>
        } else {
            <form class="chip" method="post" action=(action) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                <input type="hidden" name="channel_id" value=(id)>
                <span class="chip-label">(label)</span>
                <button type="submit" class="chip-remove" title="Remove">
                    icon(name: Icon::X)
                </button>
            </form>
        }
    })
}
