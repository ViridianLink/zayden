use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{channels, form_summary, select_row, toggle_row};
use super::provider::{Banner, banner, query_value, status_unknown};
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::confirm::confirm_button;
use crate::components::save_bar::save_bar;
use crate::guild::dto::patreon::OUTCOME_PARAM;
use crate::guild::dto::{PatreonOutcome, PatreonStatus};
use crate::guild::patreon::{
    PatreonSettingsForm,
    disconnect_patreon,
    save_patreon_settings,
};
use crate::guild::{GuildError, GuildForm};
use crate::nav::path_segment;
use crate::shell::GuildId;

const PAGE: Page = Page::Patreon;
const FORM: &str = "patreon-settings";
const DISCONNECT: &str = "patreon-disconnect";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(patreon).page(save).page(disconnect)
}

#[page("/guild/{guild_id}/patreon")]
async fn patreon(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/patreon")]
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
        message: "Patreon settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/patreon/disconnect")]
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
        message: PatreonOutcome::Disconnected.message(),
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
    let form = PatreonSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_patreon_settings(cx, &form).await
}

async fn drop_connection(
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
    state: &PageState,
) -> Result<impl View> {
    let line = query_value(cx, OUTCOME_PARAM)
        .as_deref()
        .and_then(PatreonOutcome::from_key)
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
                provider: "Patreon",
                reason: reason,
                advice: "Reload once Patreon is reachable. Connecting from here while the status is unknown would overwrite whatever campaign is already linked."
            ),
        }
    }
    .boxed())
}

#[component]
async fn panel(
    guild_id: &str,
    status: &PatreonStatus,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let connect_href = format!("/patreon/connect?guild={}", path_segment(guild_id));
    let page = PAGE.href(guild_id);
    let disconnect_action = format!("{page}/disconnect");
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
    let confirm_object = format!("from {creator}");
    let sent = state.sent(FORM);
    let dropped = state.sent(DISCONNECT);

    Ok(view! {
        if status.connected {
            <section
                class="settings-section"
                aria-labelledby="patreon-connection-title"
            >
                <h2 class="label" id="patreon-connection-title">"Connection"</h2>
                <p class="page-lead">(connected_line)</p>
                <p class="page-lead">
                    if status.webhook_registered {
                        "New posts arrive within seconds via a webhook on the creator's account, with a poll every 15 minutes as a safety net."
                    } else {
                        "No webhook is registered, so posts arrive on the 15-minute poll. Reconnecting will try again."
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
                        "Reconnect Patreon"
                    </a>
                    <form
                        id=(DISCONNECT)
                        method="post"
                        action=(disconnect_action.as_str())
                        data-pending=""
                    >
                        <input type="hidden" name="guild" value=(guild_id)>
                        confirm_button(
                            id: "patreon-disconnect-confirm",
                            label: "Disconnect",
                            prompt: "Zayden stops announcing this campaign and drops its webhook on the creator's Patreon account. Reconnecting needs the creator to authorise again.",
                            confirm: "Disconnect Patreon",
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
                        "Posts published before the first poll are absorbed rather than announced, so connecting never floods a channel with back catalogue."
                    </p>
                    select_row(
                        form: FORM,
                        name: "channel_id",
                        label: "Announcement channel",
                        selected: sent.value("channel_id", status.channel_id.as_deref()),
                        options: channels(lists, TEXT_KINDS),
                        help: Some(
                            "Leave unset to stop announcing without disconnecting the account.",
                        ),
                        error: sent.error("channel_id")
                    )
                    toggle_row(
                        form: FORM,
                        name: "public_only",
                        label: "Posts to announce",
                        value: sent.flag("public_only", status.public_only),
                        on_label: "Public posts only",
                        off_label: "Public and patron-only posts",
                        error: sent.error("public_only")
                    )
                </fieldset>
                save_bar(notice: state.notice_for(FORM))
            </form>
        } else {
            <section
                class="settings-section"
                aria-labelledby="patreon-connection-title"
            >
                <h2 class="label" id="patreon-connection-title">"Connection"</h2>
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
            </section>
        }
    }
    .boxed())
}
