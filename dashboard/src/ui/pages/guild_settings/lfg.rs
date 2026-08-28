use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::guild::SaveLfgSettings;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

#[component]
pub(crate) fn LfgTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
) -> impl IntoView {
    let save_lfg = ServerAction::<SaveLfgSettings>::new();
    let result = save_lfg.value();
    let s = settings;

    view! {
        <fieldset class="settings-section">
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_lfg>
                <input type="hidden" name="guild" value=guild_id/>
                <ChannelSelect
                    label="LFG Channel"
                    name="lfg_channel_id"
                    selected=sel(s.lfg_channel_id.as_deref())
                    channels=channels
                    kinds=TEXT_KINDS
                />
                <RoleSelect
                    label="LFG Role"
                    name="lfg_role_id"
                    selected=sel(s.lfg_role_id.as_deref())
                    roles=roles
                />
                <SettingField
                    label="LFG Scheduled Thread ID"
                    name="lfg_scheduled_thread_id"
                    value=sel(s.lfg_scheduled_thread_id.as_deref())
                />
                <SaveButton/>
            </ActionForm>
        </fieldset>
    }
}
