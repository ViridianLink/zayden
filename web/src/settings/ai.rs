use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{channels, form_summary, select_row, toggle_row};
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::AiSection;
use crate::guild::settings::{AiSettingsForm, save_ai_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Ai;
const FORM: &str = "ai-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(ai).page(save)
}

#[page("/guild/{guild_id}/ai")]
async fn ai(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/ai")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_ai(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "AI Chat settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
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
    state: &PageState,
) -> Result<impl View> {
    let sent = state.sent(FORM);

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
                <legend>"Replies"</legend>
                <p class="page-lead">
                    "With AI responses on, Zayden replies in character whenever someone mentions him."
                </p>
                toggle_row(
                    form: FORM,
                    name: "enabled",
                    label: "AI responses",
                    value: sent.flag("enabled", settings.enabled),
                    help: Some(
                        "This is the AI Chat module: the lamp above and the Overview follow it.",
                    ),
                    error: sent.error("enabled")
                )
                select_row(
                    form: FORM,
                    name: "channel_id",
                    label: "Reply only in",
                    selected: sent.value("channel_id", settings.channel_id.as_deref()),
                    options: channels(lists, TEXT_KINDS),
                    help: Some(
                        "Leave unset to let him answer anywhere he can see, or pick one channel to keep him to a single room.",
                    ),
                    error: sent.error("channel_id")
                )
                <p class="field-hint">
                    "Every reply costs a model call, so scope this to a channel you actually want him talking in."
                </p>
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
