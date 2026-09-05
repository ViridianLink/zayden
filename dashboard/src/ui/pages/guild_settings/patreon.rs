use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, PatreonStatus};
use crate::server::patreon::SavePatreonSettings;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{SaveButton, ToggleField, save_feedback};

#[component]
pub(crate) fn PatreonTab(
    guild_id: String,
    status: PatreonStatus,
    channels: Vec<ChannelInfo>,
) -> impl IntoView {
    let save = ServerAction::<SavePatreonSettings>::new();
    let result = save.value();

    let connect_href = format!("/patreon/connect?guild={guild_id}");
    let disconnect_href = format!("/patreon/disconnect?guild={guild_id}");
    let connected = status.connected;
    let disabled = status.disabled;
    let webhook = status.webhook_registered;
    let creator = status
        .creator_name
        .clone()
        .or_else(|| status.campaign_id.clone())
        .unwrap_or_else(|| "an unnamed campaign".to_owned());

    view! {
        <fieldset class="settings-section">
            {if connected {
                view! {
                    <p class="page-lead">
                        {if disabled {
                            format!(
                                "Connected to {creator}, but Patreon has rejected the \
                                 stored authorisation. Reconnect to resume \
                                 announcements.",
                            )
                        } else {
                            format!("Connected to {creator}.")
                        }}
                    </p>
                    <p class="page-lead">
                        {if webhook {
                            "New posts arrive within seconds via a webhook on the \
                             creator's account, with a poll every 15 minutes as a \
                             safety net."
                        } else {
                            "No webhook is registered, so posts arrive on the \
                             15-minute poll. Reconnecting will try again."
                        }}
                    </p>
                    <div class="settings-actions">
                        <a class="button" href=connect_href>"Reconnect Patreon"</a>
                        <form method="post" action=disconnect_href>
                            <button type="submit" class="button danger">
                                "Disconnect"
                            </button>
                        </form>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <p class="page-lead">
                        "No Patreon account is connected. The campaign's own creator \
                         has to authorise Zayden - the connection reads their posts, \
                         so nobody else can grant it."
                    </p>
                    <div class="settings-actions">
                        <a class="button" href=connect_href>"Connect Patreon"</a>
                    </div>
                }
                    .into_any()
            }}

            {connected
                .then(|| {
                    view! {
                        {move || result.get().map(save_feedback)}
                        <ActionForm action=save>
                            <input type="hidden" name="guild" value=guild_id.clone()/>
                            <ChannelSelect
                                label="Announcement Channel"
                                name="channel_id"
                                selected=sel(status.channel_id.as_deref())
                                channels=channels
                                kinds=TEXT_KINDS
                            />
                            <ToggleField
                                label="Public Posts Only"
                                name="public_only"
                                value=status.public_only
                            />
                            <SaveButton/>
                        </ActionForm>
                        <p class="page-lead">
                            "Leave the channel unset to stop announcing without \
                             disconnecting the account. Posts published before the \
                             first poll are absorbed rather than announced, so \
                             connecting never floods a channel with back catalogue."
                        </p>
                    }
                })}
        </fieldset>
    }
}
