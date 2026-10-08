use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};
use twilight_model::channel::ChannelType;

use super::fields::{channels, form_summary, select_row};
use super::state::{Done, PageState, settle};
use super::{Lists, Page, ensure_path_guild, settings_page};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::TempVoiceSection;
use crate::guild::settings::{
    CreatorChannelForm,
    TempVoiceSettingsForm,
    create_temp_voice_creator_channel,
    save_temp_voice_settings,
};
use crate::shell::GuildId;

const PAGE: Page = Page::TempVoice;
const FORM: &str = "temp-voice-settings";
const CREATE: &str = "temp-voice-create";
const CATEGORIES: &[ChannelType] = &[ChannelType::GuildCategory];
const VOICE: &[ChannelType] = &[ChannelType::GuildVoice];

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(temp_voice).page(save).page(create)
}

#[page("/guild/{guild_id}/temp-voice")]
async fn temp_voice(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/temp-voice")]
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
        message: "Temp voice settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/temp-voice/create")]
async fn create(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = create_channel(cx, guild_id, pairs).await;
    let failure = settle(cx, CREATE, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "Creator channel created.",
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
    let form = TempVoiceSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_temp_voice_settings(cx, &form).await
}

async fn create_channel(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = CreatorChannelForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    create_temp_voice_creator_channel(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &TempVoiceSection,
    lists: &Lists,
    state: &PageState,
) -> Result<impl View> {
    let saved = state.sent(FORM);
    let created = state.sent(CREATE);
    let category = settings.category.as_deref();
    let create_action = format!("{}/create", PAGE.href(guild_id));

    Ok(view! {
        <form
            id=(FORM)
            method="post"
            action=(PAGE.href(guild_id))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = saved.summary() {
                form_summary(form: FORM, message: message)
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Channels"</legend>
                <p class="page-lead">
                    "Members join the creator channel to get a voice channel of their own in the category."
                </p>
                select_row(
                    form: FORM,
                    name: "temp_voice_category",
                    label: "Category",
                    selected: saved.value("temp_voice_category", category),
                    options: channels(lists, CATEGORIES),
                    error: saved.error("temp_voice_category")
                )
                select_row(
                    form: FORM,
                    name: "temp_voice_creator_channel",
                    label: "Creator channel",
                    selected: saved.value(
                        "temp_voice_creator_channel",
                        settings.creator_channel.as_deref(),
                    ),
                    options: channels(lists, VOICE),
                    error: saved.error("temp_voice_creator_channel")
                )
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
        <form id=(CREATE) method="post" action=(create_action.as_str()) data-pending="">
            if let Some(message) = created.summary() {
                form_summary(form: CREATE, message: message, outcome: "Not created")
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <fieldset class="settings-section">
                <legend>"Create a creator channel"</legend>
                <p class="page-lead">
                    "No creator channel yet? Zayden can make one in a category and point the settings above at it."
                </p>
                select_row(
                    form: CREATE,
                    name: "temp_voice_category",
                    label: "Create it in",
                    selected: created.value("temp_voice_category", category),
                    options: channels(lists, CATEGORIES),
                    error: created.error("temp_voice_category")
                )
                <div class="form-actions">
                    <button
                        type="submit"
                        class="btn btn-secondary"
                        data-pending-label="Creating\u{2026}"
                    >
                        "Create creator channel"
                    </button>
                </div>
            </fieldset>
        </form>
    }
    .boxed())
}
