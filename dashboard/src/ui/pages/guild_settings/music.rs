use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::guild::SaveMusicSettings;
use crate::ui::components::icons::Icon;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{
    SaveButton,
    SettingField,
    ToggleField,
    save_feedback,
};

#[component]
pub(crate) fn MusicTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
) -> impl IntoView {
    let save_music = ServerAction::<SaveMusicSettings>::new();
    let result = save_music.value();
    let s = settings;

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="music"/>"Music"</legend>
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_music>
                <input type="hidden" name="guild" value=guild_id/>
                <RoleSelect
                    label="DJ Role"
                    name="dj_role_id"
                    selected=sel(s.music_dj_role_id.as_deref())
                    roles=roles
                />
                <SettingField
                    label="Auto-disconnect (seconds)"
                    name="auto_disconnect_secs"
                    value=s.music_auto_disconnect_secs
                />
                <ToggleField
                    label="Announce Now Playing"
                    name="announce_now_playing"
                    value=s.music_announce_now_playing
                />
                <ChannelSelect
                    label="Announce Channel"
                    name="announce_channel_id"
                    selected=sel(s.music_announce_channel_id.as_deref())
                    channels=channels
                    kinds=TEXT_KINDS
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "Announcements post when a track ends and the next "
                "one starts. Leave the announce channel unset to use "
                "the channel /play was run in."
            </p>
            <p class="page-lead">
                "Default volume, 24/7 mode and autoplay change "
                "while music is playing - set those in Discord "
                "with /music settings."
            </p>
        </fieldset>
    }
}
