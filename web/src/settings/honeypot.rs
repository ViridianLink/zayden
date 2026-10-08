use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};
use zayden_app::config::HoneypotSettingsRow;

use super::fields::{
    Range,
    channels,
    form_summary,
    roles,
    select_row,
    text_row,
    toggle_row,
};
use super::header::switch_module;
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::HoneypotSection;
use crate::guild::settings::{HoneypotSettingsForm, save_honeypot_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Honeypot;
const FORM: &str = "honeypot-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(honeypot).page(save).page(switch)
}

#[page("/guild/{guild_id}/honeypot")]
async fn honeypot(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/honeypot")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_honeypot(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "Honeypot settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/honeypot/module")]
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

async fn save_honeypot(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = HoneypotSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_honeypot_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &HoneypotSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(FORM);
    let purge = Range { min: 0, max: Some(HoneypotSettingsRow::MAX_PURGE_SECONDS) };

    Ok(view! {
        <form
            id=(FORM)
            method="post"
            action=(PAGE.href(guild_id))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Trap"</legend>
                <p class="page-lead">
                    "Anyone who posts in the honeypot channel is banned - which purges their recent messages server-wide - and then immediately unbanned, so a recovered account can rejoin."
                </p>
                select_row(
                    form: FORM,
                    name: "channel_id",
                    label: "Honeypot channel",
                    selected: sent.value("channel_id", settings.channel_id.as_deref()),
                    options: channels(lists, TEXT_KINDS),
                    help: Some(
                        "Leave unset to turn the trap off. Keep the channel postable by @everyone - the trap only catches spam bots that can actually reach it.",
                    ),
                    error: sent.error("channel_id")
                )
                text_row(
                    form: FORM,
                    name: "purge_seconds",
                    label: "Purge window (seconds)",
                    value: sent.value("purge_seconds", Some(&settings.purge_seconds)),
                    help: Some(
                        "How far back the ban deletes the offender's messages, across every channel. Default 86400 (24 hours); 0 keeps their history and Discord caps it at 604800 (7 days).",
                    ),
                    error: sent.error("purge_seconds"),
                    range: Some(purge)
                )
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Exemptions"</legend>
                <p class="page-lead">"The server owner is always exempt."</p>
                toggle_row(
                    form: FORM,
                    name: "exempt_admins",
                    label: "Exempt admins",
                    value: sent.flag("exempt_admins", settings.exempt_admins),
                    on_label: "Exempt",
                    off_label: "Not exempt",
                    error: sent.error("exempt_admins")
                )
                select_row(
                    form: FORM,
                    name: "exempt_role_id",
                    label: "Exempt role",
                    selected: sent.value(
                        "exempt_role_id",
                        settings.exempt_role_id.as_deref(),
                    ),
                    options: roles(lists),
                    error: sent.error("exempt_role_id")
                )
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
