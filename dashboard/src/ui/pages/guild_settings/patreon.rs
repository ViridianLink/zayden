use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use super::{TEXT_KINDS, sel};
use crate::dto::patreon::OUTCOME_PARAM;
use crate::dto::{ChannelInfo, PatreonOutcome, PatreonStatus};
use crate::server::patreon::{DisconnectPatreon, SavePatreonSettings};
use crate::ui::components::confirm::ConfirmButton;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{
    Alert,
    SaveButton,
    ToggleField,
    save_feedback,
};

#[component]
pub(crate) fn PatreonTab(
    guild_id: String,
    status: Result<PatreonStatus, String>,
    channels: Result<Vec<ChannelInfo>, String>,
    disconnect: ServerAction<DisconnectPatreon>,
) -> impl IntoView {
    let query = use_query_map();
    let outcome = move || {
        query
            .with(|q| q.get(OUTCOME_PARAM))
            .as_deref()
            .and_then(PatreonOutcome::from_key)
    };

    let panel = match status {
        Ok(status) => view! {
            <PatreonPanel
                guild_id=guild_id
                status=status
                channels=channels
                disconnect=disconnect
            />
        }
        .into_any(),
        Err(reason) => view! {
            <fieldset class="settings-section">
                <p class="warning">
                    "Couldn't load the Patreon connection: " {reason}
                </p>
                <p class="page-lead">
                    "Reload once Patreon is reachable. Connecting from here \
                     while the status is unknown would overwrite whatever \
                     campaign is already linked."
                </p>
            </fieldset>
        }
        .into_any(),
    };

    let disconnected = disconnect.value();

    view! {
        {move || {
            outcome()
                .map(|o| {
                    view! {
                        <Alert
                            class=format!("alert {}", o.class())
                            role=o.role()
                            message=o.message()
                        />
                    }
                })
        }}
        {move || disconnected.get().map(disconnect_feedback)}
        {panel}
    }
}

fn disconnect_feedback(r: Result<(), ServerFnError>) -> AnyView {
    match r {
        Ok(()) => view! {
            <Alert
                class="alert success"
                role="status"
                message=PatreonOutcome::Disconnected.message()
            />
        }
        .into_any(),
        Err(e) => view! {
            <Alert
                class="alert error"
                role="alert"
                message=format!("Failed to disconnect: {e}")
            />
        }
        .into_any(),
    }
}

#[component]
fn PatreonPanel(
    guild_id: String,
    status: PatreonStatus,
    channels: Result<Vec<ChannelInfo>, String>,
    disconnect: ServerAction<DisconnectPatreon>,
) -> impl IntoView {
    let save = ServerAction::<SavePatreonSettings>::new();
    let result = save.value();

    let connect_href = format!("/patreon/connect?guild={guild_id}");
    let disconnect_guild = guild_id.clone();
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
                        <a class="btn btn-secondary" href=connect_href>
                            "Reconnect Patreon"
                        </a>
                        <ActionForm action=disconnect>
                            <input
                                type="hidden"
                                name="guild"
                                value=disconnect_guild
                            />
                            <ConfirmButton
                                pending=disconnect.pending()
                                label="Disconnect"
                                prompt="Zayden stops announcing this campaign \
                                        and drops its webhook on the creator's \
                                        Patreon account. Reconnecting needs \
                                        the creator to authorise again."
                                confirm="Disconnect Patreon"
                            />
                        </ActionForm>
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
                        <a class="btn btn-primary" href=connect_href>
                            "Connect Patreon"
                        </a>
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
                            <SaveButton pending=save.pending()/>
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
