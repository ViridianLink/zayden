use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{channels, form_summary, select_row};
use super::provider::{Banner, banner, query_value, status_unknown};
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::confirm::confirm_button;
use crate::components::save_bar::save_bar;
use crate::guild::dto::youtube::OUTCOME_PARAM;
use crate::guild::dto::{YoutubeOutcome, YoutubeStatus};
use crate::guild::youtube::{
    YoutubeSettingsForm,
    disconnect_youtube,
    save_youtube_settings,
};
use crate::guild::{GuildError, GuildForm};
use crate::nav::path_segment;
use crate::shell::GuildId;

const PAGE: Page = Page::Youtube;
const FORM: &str = "youtube-settings";
const DISCONNECT: &str = "youtube-disconnect";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(youtube).page(save).page(disconnect)
}

#[page("/guild/{guild_id}/youtube")]
async fn youtube(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/youtube")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_settings(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "YouTube settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/youtube/disconnect")]
async fn disconnect(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = drop_connection(cx, guild_id, pairs).await;
    let failure = settle(cx, DISCONNECT, values, result, &Done {
        page: PAGE.href(guild_id),
        section: None,
        message: YoutubeOutcome::Disconnected.message(),
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

async fn save_settings(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = YoutubeSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_youtube_settings(cx, &form).await
}

async fn drop_connection(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = GuildForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    disconnect_youtube(cx, &form).await
}

#[component]
pub(super) async fn tab(
    cx: &Cx,
    guild_id: &str,
    status: &std::result::Result<YoutubeStatus, String>,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let line = query_value(cx, OUTCOME_PARAM)
        .as_deref()
        .and_then(YoutubeOutcome::from_key)
        .map(|outcome| Banner {
            class: outcome.class(),
            role: outcome.role(),
            message: outcome.message(),
        });

    Ok(view! {
        banner(line: line.as_ref())
        match status {
            Ok(status) => panel(
                guild_id: guild_id,
                status: status,
                lists: lists,
                state: state
            ),
            Err(reason) => status_unknown(
                provider: "YouTube",
                reason: reason,
                advice: "Reload before connecting - connecting while the status is unknown would replace whatever channel is already linked."
            ),
        }
    }
    .boxed())
}

#[component]
async fn panel(
    guild_id: &str,
    status: &YoutubeStatus,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let connect_href = format!("/youtube/connect?guild={}", path_segment(guild_id));
    let page = PAGE.href(guild_id);
    let disconnect_action = format!("{page}/disconnect");
    let title = status.channel_title.as_deref().unwrap_or("an unnamed channel");
    let confirm_object = format!("channel {title}");
    let sent = state.sent(FORM);
    let dropped = state.sent(DISCONNECT);

    Ok(view! {
        if status.connected {
            <section
                class="settings-section"
                aria-labelledby="youtube-connection-title"
            >
                <h2 class="label" id="youtube-connection-title">"Connection"</h2>
                <p class="page-lead">(format!("Connected to {title}."))</p>
                <p class="page-lead">
                    if status.push_active {
                        "New uploads arrive within minutes via YouTube's push notifications, with a poll every 15 minutes as a safety net."
                    } else {
                        "Push notifications are not confirmed yet, so uploads arrive on the 15-minute poll. They are renewed automatically."
                    }
                </p>
                if let Some(message) = dropped.summary() {
                    form_summary(
                        form: DISCONNECT,
                        message: message,
                        outcome: "Not disconnected"
                    )
                }
                <div class="settings-actions">
                    <a
                        class="btn btn-secondary"
                        href=(connect_href.as_str())
                        rel="external"
                    >
                        "Reconnect YouTube"
                    </a>
                    <form
                        id=(DISCONNECT)
                        method="post"
                        action=(disconnect_action.as_str())
                        data-pending=""
                    >
                        <input type="hidden" name="guild" value=(guild_id)>
                        confirm_button(
                            id: "youtube-disconnect-confirm",
                            label: "Disconnect",
                            prompt: "Zayden stops announcing this channel's uploads. Reconnecting needs the channel owner to sign in with Google again.",
                            confirm: "Disconnect YouTube",
                            object: Some(&confirm_object)
                        )
                    </form>
                </div>
            </section>
            <form
                id=(FORM)
                method="post"
                action=(page.as_str())
                data-pending=""
                data-dirty-guard=""
            >
                if let Some(message) = sent.summary() {
                    form_summary(form: FORM, message: message)
                }
                <input type="hidden" name="guild" value=(guild_id)>
                <fieldset class="settings-section">
                    <legend>"Announcements"</legend>
                    <p class="page-lead">
                        "Only public uploads are announced; videos older than two days when Zayden first sees them are skipped, so connecting never floods a channel with the back catalogue."
                    </p>
                    select_row(
                        form: FORM,
                        name: "channel_id",
                        label: "Announcement channel",
                        selected: sent.value("channel_id", status.channel_id.as_deref()),
                        options: channels(lists, TEXT_KINDS),
                        help: Some(
                            "Leave unset to stop announcing without disconnecting.",
                        ),
                        error: sent.error("channel_id")
                    )
                </fieldset>
                save_bar(notice: state.notice_for(FORM))
            </form>
        } else {
            <section
                class="settings-section"
                aria-labelledby="youtube-connection-title"
            >
                <h2 class="label" id="youtube-connection-title">"Connection"</h2>
                <p class="page-lead">
                    "No YouTube channel is connected. The channel's owner signs in with Google once to prove it is theirs; Zayden keeps no access to the account afterwards."
                </p>
                <div class="settings-actions">
                    <a
                        class="btn btn-primary"
                        href=(connect_href.as_str())
                        rel="external"
                    >
                        "Connect YouTube"
                    </a>
                </div>
            </section>
        }
    }
    .boxed())
}
