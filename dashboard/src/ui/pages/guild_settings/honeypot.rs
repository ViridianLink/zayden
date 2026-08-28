use leptos::form::ActionForm;
use leptos::prelude::*;

use super::{TEXT_KINDS, sel};
use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::guild::SaveHoneypotSettings;
use crate::ui::components::icons::Icon;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{
    SaveButton,
    SettingField,
    ToggleField,
    save_feedback,
};

#[component]
pub(crate) fn HoneypotTab(
    guild_id: String,
    settings: GuildSettings,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
) -> impl IntoView {
    let save_honeypot = ServerAction::<SaveHoneypotSettings>::new();
    let result = save_honeypot.value();
    let s = settings;

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="shield"/>"Honeypot"</legend>
            {move || result.get().map(save_feedback)}
            <ActionForm action=save_honeypot>
                <input type="hidden" name="guild" value=guild_id/>
                <ChannelSelect
                    label="Honeypot Channel"
                    name="channel_id"
                    selected=sel(s.honeypot_channel_id.as_deref())
                    channels=channels
                    kinds=TEXT_KINDS
                />
                <ToggleField
                    label="Exempt Admins"
                    name="exempt_admins"
                    value=s.honeypot_exempt_admins
                />
                <RoleSelect
                    label="Exempt Role"
                    name="exempt_role_id"
                    selected=sel(s.honeypot_exempt_role_id.as_deref())
                    roles=roles
                />
                <SettingField
                    label="Purge Window (seconds)"
                    name="purge_seconds"
                    value=s.honeypot_purge_seconds
                />
                <SaveButton/>
            </ActionForm>
            <p class="page-lead">
                "Anyone who posts in the honeypot channel is "
                "banned - which purges their recent messages "
                "server-wide - and then immediately unbanned, "
                "so a recovered account can rejoin. Leave the "
                "channel unset to turn the trap off."
            </p>
            <p class="page-lead">
                "The purge window is how far back the ban deletes "
                "the offender's messages, across every channel. "
                "Defaults to 86400 (24 hours); 0 keeps their "
                "history and Discord caps it at 604800 (7 days)."
            </p>
            <p class="page-lead">
                "The server owner is always exempt. Keep the channel "
                "postable by @everyone - the trap only catches "
                "spam bots that can actually reach it."
            </p>
        </fieldset>
    }
}
