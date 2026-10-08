use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{channels, form_summary, roles, select_row, text_row};
use super::state::{Done, PageState, settle};
use super::{Lists, Page, TEXT_KINDS, ensure_path_guild, settings_page};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::LfgSection;
use crate::guild::settings::{LfgSettingsForm, save_lfg_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Lfg;
const FORM: &str = "lfg-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(lfg).page(save)
}

#[page("/guild/{guild_id}/lfg")]
async fn lfg(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/lfg")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_lfg(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "LFG settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

async fn save_lfg(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = LfgSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_lfg_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &LfgSection,
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
                <legend>"Group finder"</legend>
                <p class="page-lead">
                    "Where looking-for-group posts go and who they ping."
                </p>
                select_row(
                    form: FORM,
                    name: "lfg_channel_id",
                    label: "LFG channel",
                    selected: sent.value(
                        "lfg_channel_id",
                        settings.channel_id.as_deref(),
                    ),
                    options: channels(lists, TEXT_KINDS),
                    error: sent.error("lfg_channel_id")
                )
                select_row(
                    form: FORM,
                    name: "lfg_role_id",
                    label: "LFG role",
                    selected: sent.value("lfg_role_id", settings.role_id.as_deref()),
                    options: roles(lists),
                    help: Some("Pinged when a group is posted."),
                    error: sent.error("lfg_role_id")
                )
                text_row(
                    form: FORM,
                    name: "lfg_scheduled_thread_id",
                    label: "Scheduled thread ID",
                    value: sent.value(
                        "lfg_scheduled_thread_id",
                        settings.scheduled_thread_id.as_deref(),
                    ),
                    help: Some(
                        "The ID of a thread in this server. Leave blank for none.",
                    ),
                    error: sent.error("lfg_scheduled_thread_id"),
                    numeric: true
                )
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
