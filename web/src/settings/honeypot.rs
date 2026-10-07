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
use crate::components::pickers::{channel_select, role_select};
use crate::components::settings::{
    save_button,
    save_feedback,
    setting_field,
    toggle_field,
};
use crate::guild::GuildError;
use crate::guild::dto::HoneypotSection;
use crate::guild::settings::{HoneypotSettingsForm, save_honeypot_settings};
use crate::shell::GuildId;

const SLUG: &str = "honeypot";
const FORM: &str = "honeypot";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(honeypot).page(save)
}

#[page("/guild/{guild_id}/settings/honeypot")]
async fn honeypot(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/honeypot")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission =
        Submission::new(FORM, values, save_honeypot(cx, guild_id, pairs).await)?;

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
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
                    label: "Honeypot Channel",
                    name: "channel_id",
                    selected: shown(
                        submitted,
                        "channel_id",
                        settings.channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                toggle_field(
                    label: "Exempt Admins",
                    name: "exempt_admins",
                    value: flag(submitted, "exempt_admins", settings.exempt_admins)
                )
                role_select(
                    label: "Exempt Role",
                    name: "exempt_role_id",
                    selected: shown(
                        submitted,
                        "exempt_role_id",
                        settings.exempt_role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                setting_field(
                    label: "Purge Window (seconds)",
                    name: "purge_seconds",
                    value: shown(
                        submitted,
                        "purge_seconds",
                        Some(settings.purge_seconds.as_str()),
                    )
                )
                save_button()
            </form>
            <p class="page-lead">
                "Anyone who posts in the honeypot channel is banned - which purges their recent messages server-wide - and then immediately unbanned, so a recovered account can rejoin. Leave the channel unset to turn the trap off."
            </p>
            <p class="page-lead">
                "The purge window is how far back the ban deletes the offender's messages, across every channel. Defaults to 86400 (24 hours); 0 keeps their history and Discord caps it at 604800 (7 days)."
            </p>
            <p class="page-lead">
                "The server owner is always exempt. Keep the channel postable by @everyone - the trap only catches spam bots that can actually reach it."
            </p>
        </fieldset>
    })
}
