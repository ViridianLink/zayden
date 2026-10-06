use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Form;
use topcoat::router::{RouterBuilder, page, path_param};
use topcoat::view::{View, component, view};

use super::{Submission, action, ensure_path_guild, settings_page, shown};
use crate::components::settings::{save_button, save_feedback, setting_field};
use crate::guild::GuildError;
use crate::guild::dto::FamilySection;
use crate::guild::settings::{FamilySettingsForm, save_family_settings};
use crate::shell::GuildId;

const SLUG: &str = "family";
const FORM: &str = "family";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(family).page(save)
}

#[page("/guild/{guild_id}/settings/family")]
async fn family(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/family")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission =
        Submission::new(FORM, values, save_family(cx, guild_id, pairs).await)?;

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
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
                setting_field(
                    label: "Max Partners",
                    name: "max_partners",
                    value: shown(
                        submitted,
                        "max_partners",
                        Some(settings.max_partners.as_str()),
                    )
                )
                save_button()
            </form>
        </fieldset>
    })
}
