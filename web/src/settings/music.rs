use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};
use zayden_app::config::MusicSettingsRow;

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
use crate::guild::dto::MusicSection;
use crate::guild::settings::{MusicSettingsForm, save_music_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Music;
const FORM: &str = "music-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(music).page(save).page(switch)
}

#[page("/guild/{guild_id}/music")]
async fn music(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/music")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_music(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "Music settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/music/module")]
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

async fn save_music(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = MusicSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_music_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &MusicSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(FORM);
    let disconnect =
        Range { min: 0, max: Some(MusicSettingsRow::MAX_AUTO_DISCONNECT_SECS) };

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
                <legend>"Playback"</legend>
                <p class="page-lead">
                    "Who controls the queue and when Zayden leaves an empty channel."
                </p>
                select_row(
                    form: FORM,
                    name: "dj_role_id",
                    label: "DJ role",
                    selected: sent.value("dj_role_id", settings.dj_role_id.as_deref()),
                    options: roles(lists),
                    error: sent.error("dj_role_id")
                )
                text_row(
                    form: FORM,
                    name: "auto_disconnect_secs",
                    label: "Auto-disconnect (seconds)",
                    value: sent.value(
                        "auto_disconnect_secs",
                        Some(&settings.auto_disconnect_secs),
                    ),
                    help: Some("From 0 to 600. Default 120."),
                    error: sent.error("auto_disconnect_secs"),
                    range: Some(disconnect)
                )
            </fieldset>
            <fieldset class="settings-section">
                <legend>"Now playing"</legend>
                <p class="page-lead">
                    "Announcements post when a track ends and the next one starts."
                </p>
                toggle_row(
                    form: FORM,
                    name: "announce_now_playing",
                    label: "Announce now playing",
                    value: sent.flag(
                        "announce_now_playing",
                        settings.announce_now_playing,
                    ),
                    error: sent.error("announce_now_playing")
                )
                select_row(
                    form: FORM,
                    name: "announce_channel_id",
                    label: "Announce channel",
                    selected: sent.value(
                        "announce_channel_id",
                        settings.announce_channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    help: Some("Leave unset to use the channel /play was run in."),
                    error: sent.error("announce_channel_id")
                )
                <p class="field-hint">
                    "Default volume, 24/7 mode and autoplay change while music is playing - set those in Discord with /music settings."
                </p>
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
