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
use crate::components::select::{channel_select, role_select};
use crate::components::settings::{
    save_button,
    save_feedback,
    setting_field,
    toggle_field,
};
use crate::guild::GuildError;
use crate::guild::dto::MusicSection;
use crate::guild::settings::{MusicSettingsForm, save_music_settings};
use crate::shell::GuildId;

const SLUG: &str = "music";
const FORM: &str = "music";

pub(super) fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(music).page(save)
}

#[page("/guild/{guild_id}/settings/music")]
async fn music(cx: &Cx) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);

    Ok(view! { settings_page(guild_id: guild_id, slug: SLUG) })
}

#[page(POST "/guild/{guild_id}/settings/music")]
async fn save(
    cx: &Cx,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<impl View> {
    let guild_id: &str = path_param::<GuildId>(cx);
    let values = pairs.clone();
    let submission =
        Submission::new(FORM, values, save_music(cx, guild_id, pairs).await)?;

    Ok(view! {
        (submission.status())
        settings_page(guild_id: guild_id, slug: SLUG, submission: Some(&submission))
    })
}

async fn save_music(
    cx: &Cx,
    guild_id: &str,
    pairs: Vec<(String, String)>,
) -> std::result::Result<(), GuildError> {
    let form = MusicSettingsForm::from_pairs(pairs)?;
    ensure_path_guild(&form.guild, guild_id)?;
    save_music_settings(cx, &form).await
}

#[component]
pub(super) async fn tab(
    guild_id: &str,
    settings: &MusicSection,
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
                role_select(
                    label: "DJ Role",
                    name: "dj_role_id",
                    selected: shown(
                        submitted,
                        "dj_role_id",
                        settings.dj_role_id.as_deref(),
                    ),
                    roles: lists.roles()
                )
                setting_field(
                    label: "Auto-disconnect (seconds)",
                    name: "auto_disconnect_secs",
                    value: shown(
                        submitted,
                        "auto_disconnect_secs",
                        Some(settings.auto_disconnect_secs.as_str()),
                    )
                )
                toggle_field(
                    label: "Announce Now Playing",
                    name: "announce_now_playing",
                    value: flag(
                        submitted,
                        "announce_now_playing",
                        settings.announce_now_playing,
                    )
                )
                channel_select(
                    label: "Announce Channel",
                    name: "announce_channel_id",
                    selected: shown(
                        submitted,
                        "announce_channel_id",
                        settings.announce_channel_id.as_deref(),
                    ),
                    channels: lists.channels(),
                    kinds: TEXT_KINDS
                )
                save_button()
            </form>
            <p class="page-lead">
                "Announcements post when a track ends and the next one starts. Leave the announce channel unset to use the channel /play was run in."
            </p>
            <p class="page-lead">
                "Default volume, 24/7 mode and autoplay change while music is playing - set those in Discord with /music settings."
            </p>
        </fieldset>
    })
}
