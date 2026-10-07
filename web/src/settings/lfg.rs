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
    settings_page,
    shown,
};
use crate::components::pickers::{channel_select, role_select};
use crate::components::settings::{save_button, save_feedback, setting_field};
use crate::guild::GuildError;
use crate::guild::dto::LfgSection;
use crate::guild::settings::{LfgSettingsForm, save_lfg_settings};
use crate::shell::GuildId;

const SLUG: &str = "lfg";
const FORM: &str = "lfg";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(lfg).page(save)
}

#[page("/guild/{guild_id}/settings/lfg")]
async fn lfg(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/lfg")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission =
        Submission::new(FORM, values, save_lfg(cx, guild_id, pairs).await)?;

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
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
                channel_select(
                    label: "LFG Channel",
                    name: "lfg_channel_id",
                    selected: shown(
                        submitted,
                        "lfg_channel_id",
                        settings.channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                role_select(
                    label: "LFG Role",
                    name: "lfg_role_id",
                    selected: shown(
                        submitted,
                        "lfg_role_id",
                        settings.role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                setting_field(
                    label: "LFG Scheduled Thread ID",
                    name: "lfg_scheduled_thread_id",
                    value: shown(
                        submitted,
                        "lfg_scheduled_thread_id",
                        settings.scheduled_thread_id.as_deref(),
                    )
                )
                save_button()
            </form>
        </fieldset>
    })
}
