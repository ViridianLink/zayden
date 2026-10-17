use ::deadlock::announce::{DEFAULT_MESSAGE, PLACEHOLDERS};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::announce::{
    ADD_PING_ROLE,
    REMOVE_PING_ROLE,
    ping_roles_section,
    settle_ping_role,
};
use super::fields::channels;
use super::header::switch_module;
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::field_row::{
    form_summary,
    select_row,
    textarea_row,
    toggle_row,
};
use crate::components::placeholder_legend::placeholder_legend;
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::announce::PingRoleForm;
use crate::guild::deadlock::{
    DeadlockSettingsForm,
    DeadlockSquadForm,
    DeadlockSummariesForm,
    add_deadlock_ping_role,
    remove_deadlock_ping_role,
    save_deadlock_settings,
    save_deadlock_squad,
    save_deadlock_summaries,
};
use crate::guild::dto::DeadlockSection;
use crate::shell::GuildId;

const PAGE: Page = Page::Deadlock;
const FORM: &str = "deadlock-settings";
const SUMMARIES_FORM: &str = "deadlock-summaries";
const SQUAD_FORM: &str = "deadlock-squad";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(deadlock)
        .page(save)
        .page(switch)
        .page(save_summaries)
        .page(save_squad)
        .page(add_ping_role)
        .page(remove_ping_role)
}

#[page("/guild/{guild_id}/deadlock")]
async fn deadlock(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/deadlock")]
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
        message: "Deadlock settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/deadlock/module")]
async fn switch(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::failed(switch_module(cx, guild_id, PAGE, pairs).await?);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/deadlock/summaries")]
async fn save_summaries(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = DeadlockSummariesForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_deadlock_summaries(cx, &form).await
    }
    .await;
    let failure = settle(cx, SUMMARIES_FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(SUMMARIES_FORM),
        message: "Match summary settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/deadlock/squad")]
async fn save_squad(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = DeadlockSquadForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        save_deadlock_squad(cx, &form).await
    }
    .await;
    let failure = settle(cx, SQUAD_FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(SQUAD_FORM),
        message: "Squad tracking settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/deadlock/add-ping-role")]
async fn add_ping_role(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = PingRoleForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        add_deadlock_ping_role(cx, &form).await
    }
    .await;
    let state = settle_ping_role(cx, PAGE, guild_id, ADD_PING_ROLE, values, result)?;

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/deadlock/remove-ping-role")]
async fn remove_ping_role(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = async {
        let form = PingRoleForm::from_pairs(pairs)?;
        ensure_path_guild(&form.guild, guild_id)?;
        remove_deadlock_ping_role(cx, &form).await
    }
    .await;
    let state =
        settle_ping_role(cx, PAGE, guild_id, REMOVE_PING_ROLE, values, result)?;

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
    let form = DeadlockSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_deadlock_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &DeadlockSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let page = PAGE.href(guild_id);
    let sent = state.sent(FORM);
    let summaries_sent = state.sent(SUMMARIES_FORM);
    let squad_sent = state.sent(SQUAD_FORM);
    let default_help = format!(
        "Leave empty for the default: {}",
        DEFAULT_MESSAGE.replace('\n', " then ")
    );

    Ok(view! {
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
                <legend>"Patch notes"</legend>
                <p class="page-lead">
                    "When the official patch notes are posted, Zayden announces them here with the hero and item changes found between the old and new game builds."
                </p>
                select_row(
                    form: FORM,
                    name: "channel_id",
                    label: "Announcement channel",
                    selected: sent.value("channel_id", settings.channel_id.as_deref()),
                    options: channels(lists, TEXT_KINDS),
                    help: Some(
                        "Leave unset to stop announcing. This also clears the message and roles below.",
                    ),
                    error: sent.error("channel_id")
                )
                textarea_row(
                    form: FORM,
                    name: "message",
                    label: "Message",
                    value: sent.value("message", settings.message.as_deref()),
                    rows: 4,
                    help: Some(&default_help),
                    error: sent.error("message")
                )
                placeholder_legend(entries: PLACEHOLDERS)
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
        ping_roles_section(
            guild_id: guild_id,
            page: PAGE,
            configured: &settings.ping_role_ids,
            has_channel: settings.channel_id.is_some(),
            lists: lists,
            state: state
        )
        <form
            id=(SUMMARIES_FORM)
            method="post"
            action=(format!("{page}/summaries"))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = summaries_sent.summary() {
                form_summary(form: SUMMARIES_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Match summaries"</legend>
                <p class="page-lead">
                    "After a match, members who linked Steam and chose to share summaries in this server get a recap posted here."
                </p>
                select_row(
                    form: SUMMARIES_FORM,
                    name: "summary_channel_id",
                    label: "Summary channel",
                    selected: summaries_sent.value(
                        "summary_channel_id",
                        settings.summary_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    help: Some("Leave unset to stop posting summaries here."),
                    error: summaries_sent.error("summary_channel_id")
                )
            </fieldset>
            save_bar(notice: state.notice_for(SUMMARIES_FORM))
        </form>
        <form
            id=(SQUAD_FORM)
            method="post"
            action=(format!("{page}/squad"))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = squad_sent.summary() {
                form_summary(form: SQUAD_FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Squad tracking"</legend>
                <p class="page-lead">
                    "Members who link Steam and run `/deadlock squad join` get rank changes and win/loss streaks posted here."
                </p>
                select_row(
                    form: SQUAD_FORM,
                    name: "squad_channel_id",
                    label: "Squad channel",
                    selected: squad_sent.value(
                        "squad_channel_id",
                        settings.squad_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    help: Some("Leave unset to stop posting squad updates here."),
                    error: squad_sent.error("squad_channel_id")
                )
                toggle_row(
                    form: SQUAD_FORM,
                    name: "squad_leaderboard",
                    label: "Post a weekly leaderboard (Sundays)",
                    value: squad_sent.flag(
                        "squad_leaderboard",
                        settings.squad_leaderboard,
                    ),
                    error: squad_sent.error("squad_leaderboard")
                )
            </fieldset>
            save_bar(notice: state.notice_for(SQUAD_FORM))
        </form>
    }
    .boxed())
}
