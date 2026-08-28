use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::guild::{SaveChannelSettings, SaveRoleSettings};
use crate::ui::components::icons::Icon;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{SaveButton, save_feedback};

#[component]
pub(crate) fn GeneralTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
) -> impl IntoView {
    let save_channels = ServerAction::<SaveChannelSettings>::new();
    let save_roles = ServerAction::<SaveRoleSettings>::new();
    let channel_result = save_channels.value();
    let role_result = save_roles.value();

    let s = settings;
    let gid = guild_id.clone();

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="grid"/>"Channels"</legend>
            {move || channel_result.get().map(save_feedback)}
            <ActionForm action=save_channels>
                <input type="hidden" name="guild" value=gid/>
                <ChannelSelect
                    label="Rules Channel"
                    name="rules_channel_id"
                    selected=sel(s.rules_channel_id.as_deref())
                    channels=channels.clone()
                    kinds=TEXT_KINDS
                />
                <ChannelSelect
                    label="General Channel"
                    name="general_channel_id"
                    selected=sel(s.general_channel_id.as_deref())
                    channels=channels.clone()
                    kinds=TEXT_KINDS
                />
                <ChannelSelect
                    label="Spoiler Channel"
                    name="spoiler_channel_id"
                    selected=sel(s.spoiler_channel_id.as_deref())
                    channels=channels
                    kinds=TEXT_KINDS
                />
                <SaveButton/>
            </ActionForm>
        </fieldset>

        <fieldset class="settings-section">
            <legend><Icon name="users"/>"Roles"</legend>
            {move || role_result.get().map(save_feedback)}
            <ActionForm action=save_roles>
                <input type="hidden" name="guild" value=guild_id/>
                <RoleSelect
                    label="Artist Role"
                    name="artist_role_id"
                    selected=sel(s.artist_role_id.as_deref())
                    roles=roles.clone()
                />
                <RoleSelect
                    label="Sleep Role"
                    name="sleep_role_id"
                    selected=sel(s.sleep_role_id.as_deref())
                    roles=roles.clone()
                />
                <RoleSelect
                    label="Verified Role"
                    name="verified_role_id"
                    selected=sel(s.verified_role_id.as_deref())
                    roles=roles
                />
                <SaveButton/>
            </ActionForm>
        </fieldset>
    }
}
