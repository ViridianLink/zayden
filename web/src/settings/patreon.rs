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
    flag,
    settings_page,
    shown,
};
use crate::components::confirm::confirm_button;
use crate::components::select::channel_select;
use crate::components::settings::{save_button, save_feedback, toggle_field};
use crate::guild::dto::patreon::OUTCOME_PARAM;
use crate::guild::dto::{PatreonOutcome, PatreonStatus};
use crate::guild::patreon::{
    PatreonSettingsForm,
    disconnect_patreon,
    save_patreon_settings,
};
use crate::guild::{GuildError, GuildForm};
use crate::shell::GuildId;

const SLUG: &str = "patreon";
const SAVE: &str = "save";
const DISCONNECT: &str = "disconnect";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(patreon).page(submit)
}

#[page("/guild/{guild_id}/settings/patreon")]
async fn patreon(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let submission = disconnected(cx).then(|| Submission::succeeded(DISCONNECT));

    Ok(view! {
        settings_page(guild_id: guild_id, slug: SLUG, submission: submission.as_ref())
    })
}

#[page(POST "/guild/{guild_id}/settings/patreon")]
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
    let form = PatreonSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_patreon_settings(cx, &form).await
}

async fn disconnect(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = GuildForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    disconnect_patreon(cx, &form).await
}

#[component]
pub(super) async fn tab(
    cx: &Cx,
    guild_id: &str,
    status: &std::result::Result<PatreonStatus, String>,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let banner = query_value(cx, OUTCOME_PARAM)
        .as_deref()
        .and_then(PatreonOutcome::from_key)
        .map(|outcome| {
            Notice::outcome(outcome.class(), outcome.role(), outcome.message())
        });
    let disconnected = Submission::of(submission, DISCONNECT).map(|submitted| {
        Notice::disconnect(
            submitted.outcome(),
            PatreonOutcome::Disconnected.message(),
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
                    "Couldn't load the Patreon connection: "
                    (reason)
                </p>
                <p class="page-lead">
                    "Reload once Patreon is reachable. Connecting from here while the status is unknown would overwrite whatever campaign is already linked."
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
    status: &PatreonStatus,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let connect_href = format!("/patreon/connect?guild={guild_id}");
    let creator = status
        .creator_name
        .as_deref()
        .or(status.campaign_id.as_deref())
        .unwrap_or("an unnamed campaign");
    let connected_line = if status.disabled {
        format!(
            "Connected to {creator}, but Patreon has rejected the stored \
             authorisation. Reconnect to resume announcements."
        )
    } else {
        format!("Connected to {creator}.")
    };

    Ok(view! {
        <fieldset class="settings-section">
            if status.connected {
                <p class="page-lead">(connected_line)</p>
                <p class="page-lead">
                    if status.webhook_registered {
                        "New posts arrive within seconds via a webhook on the creator's account, with a poll every 15 minutes as a safety net."
                    } else {
                        "No webhook is registered, so posts arrive on the 15-minute poll. Reconnecting will try again."
                    }
                </p>
                <div class="settings-actions">
                    <a
                        class="btn btn-secondary"
                        href=(connect_href.as_str())
                        rel="external"
                    >
                        "Reconnect Patreon"
                    </a>
                    <form method="post" action=(action) data-pending="">
                        <input type="hidden" name="guild" value=(guild_id)>
                        confirm_button(
                            label: "Disconnect",
                            prompt: "Zayden stops announcing this campaign and drops its webhook on the creator's Patreon account. Reconnecting needs the creator to authorise again.",
                            confirm: "Disconnect Patreon"
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
                    toggle_field(
                        label: "Public Posts Only",
                        name: "public_only",
                        value: flag(submission, "public_only", status.public_only)
                    )
                    save_button()
                </form>
                <p class="page-lead">
                    "Leave the channel unset to stop announcing without disconnecting the account. Posts published before the first poll are absorbed rather than announced, so connecting never floods a channel with back catalogue."
                </p>
            } else {
                <p class="page-lead">
                    "No Patreon account is connected. The campaign's own creator has to authorise Zayden - the connection reads their posts, so nobody else can grant it."
                </p>
                <div class="settings-actions">
                    <a
                        class="btn btn-primary"
                        href=(connect_href.as_str())
                        rel="external"
                    >
                        "Connect Patreon"
                    </a>
                </div>
            }
        </fieldset>
    })
}
