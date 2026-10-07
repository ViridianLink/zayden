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
use crate::components::icons::{Icon, icon};
use crate::components::pickers::{channel_select, role_select};
use crate::components::settings::{save_button, save_feedback};
use crate::guild::GuildError;
use crate::guild::dto::GeneralSection;
use crate::guild::settings::{
    ChannelSettingsForm,
    RoleSettingsForm,
    save_channel_settings,
    save_role_settings,
};
use crate::shell::GuildId;

const SLUG: &str = "general";
const CHANNELS: &str = "channels";
const ROLES: &str = "roles";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(general).page(save)
}

#[page("/guild/{guild_id}/settings/general")]
async fn general(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/general")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission = if pairs.iter().any(|(name, _)| name == "rules_channel_id") {
        Submission::new(CHANNELS, values, save_channels(cx, guild_id, pairs).await)?
    } else {
        Submission::new(ROLES, values, save_roles(cx, guild_id, pairs).await)?
    };

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
    })
}

async fn save_channels(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = ChannelSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_channel_settings(cx, &form).await
}

async fn save_roles(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = RoleSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_role_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &GeneralSection,
    lists: &Lists,
    submission: Option<&Submission>,
) -> Result<impl View> {
    let action = action(guild_id, SLUG);
    let channels = Submission::of(submission, CHANNELS);
    let roles = Submission::of(submission, ROLES);

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Grid)
                "Channels"
            </legend>
            if let Some(submitted) = channels {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action.as_str()) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                channel_select(
                    label: "Rules Channel",
                    name: "rules_channel_id",
                    selected: shown(
                        channels,
                        "rules_channel_id",
                        settings.rules_channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                channel_select(
                    label: "General Channel",
                    name: "general_channel_id",
                    selected: shown(
                        channels,
                        "general_channel_id",
                        settings.general_channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                channel_select(
                    label: "Spoiler Channel",
                    name: "spoiler_channel_id",
                    selected: shown(
                        channels,
                        "spoiler_channel_id",
                        settings.spoiler_channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                save_button()
            </form>
        </fieldset>
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Users)
                "Roles"
            </legend>
            if let Some(submitted) = roles {
                save_feedback(outcome: submitted.outcome())
            }
            <form method="post" action=(action.as_str()) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                role_select(
                    label: "Artist Role",
                    name: "artist_role_id",
                    selected: shown(
                        roles,
                        "artist_role_id",
                        settings.artist_role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                role_select(
                    label: "Sleep Role",
                    name: "sleep_role_id",
                    selected: shown(
                        roles,
                        "sleep_role_id",
                        settings.sleep_role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                role_select(
                    label: "Verified Role",
                    name: "verified_role_id",
                    selected: shown(
                        roles,
                        "verified_role_id",
                        settings.verified_role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                save_button()
            </form>
        </fieldset>
    })
}
