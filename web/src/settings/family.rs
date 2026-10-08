use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, ViewExt, component, view};

use super::fields::{Range, form_summary, text_row};
use super::header::switch_module;
use super::state::{Done, PageState, settle};
use super::{Page, ensure_path_guild, settings_page};
use crate::components::save_bar::save_bar;
use crate::guild::GuildError;
use crate::guild::dto::FamilySection;
use crate::guild::settings::{FamilySettingsForm, save_family_settings};
use crate::shell::GuildId;

const PAGE: Page = Page::Family;
const FORM: &str = "family-settings";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(family).page(save).page(switch)
}

#[page("/guild/{guild_id}/family")]
async fn family(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let state = PageState::load(cx);

    Ok(view! { settings_page(guild_id: guild_id, page: PAGE, state: &state) })
}

#[page(POST "/guild/{guild_id}/family")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let result = save_family(cx, guild_id, pairs).await;
    let failure = settle(cx, FORM, values, result, &Done {
        page: PAGE.href(guild_id),
        section: Some(FORM),
        message: "Family settings saved.",
    })?;
    let state = PageState::failed(failure);

    Ok(view! {
        (state.status())
        settings_page(guild_id: guild_id, page: PAGE, state: &state)
    })
}

#[page(POST "/guild/{guild_id}/family/module")]
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

async fn save_family(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = FamilySettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_family_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &FamilySection,
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
                <legend>"Relationships"</legend>
                <p class="page-lead">
                    "How many partners one member can have at once."
                </p>
                text_row(
                    form: FORM,
                    name: "max_partners",
                    label: "Max partners",
                    value: sent.value("max_partners", Some(&settings.max_partners)),
                    help: Some("At least 1."),
                    error: sent.error("max_partners"),
                    range: Some(Range { min: 1, max: None })
                )
            </fieldset>
            save_bar(notice: state.notice_for(FORM))
        </form>
    }
    .boxed())
}
