use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, component, view};

use super::{
    Lists,
    Submission,
    TEXT_KINDS,
    action,
    ensure_path_guild,
    flag,
    settings_page,
    shown,
};
use crate::components::pickers::channel_select;
use crate::components::settings::{save_button, save_feedback, toggle_field};
use crate::guild::GuildError;
use crate::guild::dto::AiSection;
use crate::guild::settings::{AiSettingsForm, save_ai_settings};
use crate::shell::GuildId;

const SLUG: &str = "ai";
const FORM: &str = "ai";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(ai).page(save)
}

#[page("/guild/{guild_id}/settings/ai")]
async fn ai(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/ai")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission =
        Submission::new(FORM, values, save_ai(cx, guild_id, pairs).await)?;

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
    })
}

async fn save_ai(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = AiSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_ai_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &AiSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let submitted = Submission::of(submission, FORM);

    Ok(view! {
        <fieldset class="settings-section">
            if let Some(submitted) = submitted {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action(guild_id, SLUG)) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                toggle_field(
                    label: "AI Responses",
                    name: "enabled",
                    value: flag(submitted, "enabled", settings.enabled)
                )
                channel_select(
                    label: "Restrict to Channel",
                    name: "channel_id",
                    selected: shown(
                        submitted,
                        "channel_id",
                        settings.channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                save_button()
            </form>
            <p class="page-lead">
                "With AI responses on, Zayden replies in character whenever someone mentions him. Leave the channel unset to let him answer anywhere he can see, or pick one to keep him to a single room."
            </p>
            <p class="page-lead">
                "Every reply costs a model call, so scope this to a channel you actually want him talking in. The toggle here is the same switch as the AI Chat card on the Modules page."
            </p>
        </fieldset>
    })
}
