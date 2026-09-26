use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use super::{TEXT_KINDS, sel};
use crate::dto::youtube::OUTCOME_PARAM;
use crate::dto::{ChannelInfo, YoutubeOutcome, YoutubeStatus};
use crate::server::youtube::{DisconnectYoutube, SaveYoutubeSettings};
use crate::ui::components::confirm::ConfirmButton;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{Alert, SaveButton, save_feedback};

#[component]
pub(crate) fn YoutubeTab(
    guild_id: String,
    status: Result<YoutubeStatus, String>,
    channels: Result<Vec<ChannelInfo>, String>,
    disconnect: ServerAction<DisconnectYoutube>,
) -> impl IntoView {
    let query = use_query_map();
    let outcome = move || {
        query
            .with(|q| q.get(OUTCOME_PARAM))
            .as_deref()
            .and_then(YoutubeOutcome::from_key)
    };

    let panel = match status {
        Ok(status) => view! {
            <YoutubePanel
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
                    "Couldn't load the YouTube connection: " {reason}
                </p>
                <p class="page-lead">
                    "Reload before connecting - connecting while the status is \
                     unknown would replace whatever channel is already linked."
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
                message=YoutubeOutcome::Disconnected.message()
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
fn YoutubePanel(
    guild_id: String,
    status: YoutubeStatus,
    channels: Result<Vec<ChannelInfo>, String>,
    disconnect: ServerAction<DisconnectYoutube>,
) -> impl IntoView {
    let save = ServerAction::<SaveYoutubeSettings>::new();
    let result = save.value();

    let connect_href = format!("/youtube/connect?guild={guild_id}");
    let disconnect_guild = guild_id.clone();
    let connected = status.connected;
    let push = status.push_active;
    let title = status
        .channel_title
        .clone()
        .unwrap_or_else(|| "an unnamed channel".to_owned());

    view! {
        <fieldset class="settings-section">
            {if connected {
                view! {
                    <p class="page-lead">{format!("Connected to {title}.")}</p>
                    <p class="page-lead">
                        {if push {
                            "New uploads arrive within minutes via YouTube's push \
                             notifications, with a poll every 15 minutes as a \
                             safety net."
                        } else {
                            "Push notifications are not confirmed yet, so uploads \
                             arrive on the 15-minute poll. They are renewed \
                             automatically."
                        }}
                    </p>
                    <div class="settings-actions">
                        <a class="btn btn-secondary" href=connect_href rel="external">
                            "Reconnect YouTube"
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
                                prompt="Zayden stops announcing this channel's \
                                        uploads. Reconnecting needs the channel \
                                        owner to sign in with Google again."
                                confirm="Disconnect YouTube"
                            />
                        </ActionForm>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <p class="page-lead">
                        "No YouTube channel is connected. The channel's owner signs \
                         in with Google once to prove it is theirs; Zayden keeps no \
                         access to the account afterwards."
                    </p>
                    <div class="settings-actions">
                        <a class="btn btn-primary" href=connect_href rel="external">
                            "Connect YouTube"
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
                            <SaveButton pending=save.pending()/>
                        </ActionForm>
                        <p class="page-lead">
                            "Leave the channel unset to stop announcing without \
                             disconnecting. Only public uploads are announced; \
                             videos older than two days when Zayden first sees \
                             them are skipped, so connecting never floods a \
                             channel with the back catalogue."
                        </p>
                    }
                })}
        </fieldset>
    }
}
