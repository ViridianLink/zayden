use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::error::see_other;
use topcoat::router::request::uri;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, component, view};
use twilight_model::channel::ChannelType;
use url::form_urlencoded;

use super::{Lists, Submission, action, ensure_path_guild, settings_page, shown};
use crate::components::select::channel_select;
use crate::components::settings::{create_feedback, save_button, save_feedback};
use crate::guild::GuildError;
use crate::guild::dto::TempVoiceSection;
use crate::guild::settings::{
    CreatorChannelForm,
    TempVoiceSettingsForm,
    create_temp_voice_creator_channel,
    save_temp_voice_settings,
};
use crate::shell::GuildId;

const SLUG: &str = "temp-voice";
const SAVE: &str = "save";
const CREATE: &str = "create";
const CREATED_QUERY: &str = "created=1";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(temp_voice).page(save)
}

#[page("/guild/{guild_id}/settings/temp-voice")]
async fn temp_voice(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let created =
        form_urlencoded::parse(uri(cx).query().unwrap_or_default().as_bytes())
            .any(|(key, value)| key == "created" && value == "1");
    let submission = created.then(|| Submission::succeeded(CREATE));

    Ok(view! {
        settings_page(guild_id: guild_id, slug: SLUG, submission: submission.as_ref())
    })
}

#[page(POST "/guild/{guild_id}/settings/temp-voice")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let submission =
        if pairs.iter().any(|(name, _)| name == "temp_voice_creator_channel") {
            let values = pairs.clone();
            Submission::new(SAVE, values, save_settings(cx, guild_id, pairs).await)?
        } else {
            let created = create(cx, guild_id, pairs).await;
            if created.is_ok() {
                let location = format!("{}?{CREATED_QUERY}", action(guild_id, SLUG));
                return Err(see_other(location).into());
            }
            Submission::reloading(CREATE, created)?
        };

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
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

async fn create(
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
    submission: Option<&Submission>,
) -> Result<impl View> {
    let action = action(guild_id, SLUG);
    let saved = Submission::of(submission, SAVE);
    let created = Submission::of(submission, CREATE);
    let category = settings.category.as_deref();

    Ok(view! {
        <fieldset class="settings-section">
            if let Some(submitted) = saved {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action.as_str()) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                channel_select(
                    label: "Category",
                    name: "temp_voice_category",
                    selected: shown(saved, "temp_voice_category", category),
                    channels: lists.channels(),
                    kinds: &[ChannelType::GuildCategory]
                )
                channel_select(
                    label: "Creator Channel",
                    name: "temp_voice_creator_channel",
                    selected: shown(
                        saved,
                        "temp_voice_creator_channel",
                        settings.creator_channel.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: &[ChannelType::GuildVoice]
                )
                save_button()
            </form>
            <p class="page-lead">
                "No creator channel yet? Zayden can make one for you and point the settings above at it."
            </p>
            if let Some(submitted) = created {
                create_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action.as_str()) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                channel_select(
                    label: "Create Creator Channel In",
                    name: "temp_voice_category",
                    selected: category.unwrap_or_default(),
                    channels: lists.channels(),
                    kinds: &[ChannelType::GuildCategory]
                )
                <div class="form-actions">
                    <button type="submit" class="btn btn-secondary">
                        "Create Creator Channel"
                    </button>
                </div>
            </form>
        </fieldset>
    })
}
