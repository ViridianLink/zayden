use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::provider::{
    Notice,
    disconnected,
    disconnected_location,
    notices,
    provider_action,
    query_value,
};
use super::{
    Lists,
    Submission,
    TEXT_KINDS,
    ensure_path_guild,
    settings_page,
    shown,
};
use crate::components::confirm::confirm_button;
use crate::components::pickers::channel_select;
use crate::components::settings::{save_button, save_feedback};
use crate::guild::dto::youtube::OUTCOME_PARAM;
use crate::guild::dto::{YoutubeOutcome, YoutubeStatus};
use crate::guild::youtube::{
    YoutubeSettingsForm,
    disconnect_youtube,
    save_youtube_settings,
};
use crate::guild::{GuildError, GuildForm};
use crate::shell::GuildId;

const SLUG: &str = "youtube";
const SAVE: &str = "save";
const DISCONNECT: &str = "disconnect";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(youtube).page(submit)
}

#[page("/guild/{guild_id}/settings/youtube")]
async fn youtube(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let submission = disconnected(cx).then(|| Submission::succeeded(DISCONNECT));

    Ok(view! {
        settings_page(guild_id: guild_id, slug: SLUG, submission: submission.as_ref())
    })
}

#[page(POST "/guild/{guild_id}/settings/youtube")]
async fn submit(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let submission = if pairs.iter().any(|(name, _)| name == "channel_id") {
        Submission::new(SAVE, pairs.clone(), save(cx, guild_id, pairs).await)?
    } else {
        let result = disconnect(cx, guild_id, pairs).await;
        if result.is_ok() {
            return Err(see_other(disconnected_location(cx, guild_id, SLUG)).into());
        }
        Submission::reloading(DISCONNECT, result)?
    };

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
    })
}

async fn save(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = YoutubeSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_youtube_settings(cx, &form).await
}

async fn disconnect(
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
    submission: Option<&Submission>,
) -> Result<impl View> {
    let banner = query_value(cx, OUTCOME_PARAM)
        .as_deref()
        .and_then(YoutubeOutcome::from_key)
        .map(|outcome| {
            Notice::outcome(outcome.class(), outcome.role(), outcome.message())
        });
    let disconnected = Submission::of(submission, DISCONNECT).map(|submitted| {
        Notice::disconnect(
            submitted.outcome(),
            YoutubeOutcome::Disconnected.message(),
        )
    });
    let lines: Vec<Notice> = banner.into_iter().chain(disconnected).collect();
    let action = provider_action(cx, guild_id, SLUG);

    Ok(view! {
        notices(lines: &lines)
        match status {
            Ok(status) => panel(
                guild_id: guild_id,
                action: &action,
                status: status,
                lists: lists,
                submission: Submission::of(submission, SAVE)
            ),
            Err(reason) => <fieldset class="settings-section">
                <p class="warning">
                    "Couldn't load the YouTube connection: "
                    (reason)
                </p>
                <p class="page-lead">
                    "Reload before connecting - connecting while the status is unknown would replace whatever channel is already linked."
                </p>
            </fieldset>,
        }
    }
    .boxed())
}

#[component]
async fn panel(
    guild_id: &str,
    action: &str,
    status: &YoutubeStatus,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let connect_href = format!("/youtube/connect?guild={guild_id}");
    let title = status.channel_title.as_deref().unwrap_or("an unnamed channel");

    Ok(view! {
        <fieldset class="settings-section">
            if status.connected {
                <p class="page-lead">(format!("Connected to {title}."))</p>
                <p class="page-lead">
                    if status.push_active {
                        "New uploads arrive within minutes via YouTube's push notifications, with a poll every 15 minutes as a safety net."
                    } else {
                        "Push notifications are not confirmed yet, so uploads arrive on the 15-minute poll. They are renewed automatically."
                    }
                </p>
                <div class="settings-actions">
                    <a
                        class="btn btn-secondary"
                        href=(connect_href.as_str())
                        rel="external"
                    >
                        "Reconnect YouTube"
                    </a>
                    <form method="post" action=(action) data-pending="">
                        <input type="hidden" name="guild" value=(guild_id)>
                        confirm_button(
                            id: "youtube-disconnect",
                            label: "Disconnect",
                            prompt: "Zayden stops announcing this channel's uploads. Reconnecting needs the channel owner to sign in with Google again.",
                            confirm: "Disconnect YouTube"
                        )
                    </form>
                </div>
                if let Some(submitted) = submission {
                    save_feedback(outcome: submitted.outcome())
                }
                <form method="post" action=(action) data-pending="">
                    <input type="hidden" name="guild" value=(guild_id)>
                    channel_select(
                        label: "Announcement Channel",
                        name: "channel_id",
                        selected: shown(
                            submission,
                            "channel_id",
                            status.channel_id.as_deref(),
                        ),
                        channels: lists.channels(),
                        kinds: TEXT_KINDS
                    )
                    save_button()
                </form>
                <p class="page-lead">
                    "Leave the channel unset to stop announcing without disconnecting. Only public uploads are announced; videos older than two days when Zayden first sees them are skipped, so connecting never floods a channel with the back catalogue."
                </p>
            } else {
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
            }
        </fieldset>
    })
}
