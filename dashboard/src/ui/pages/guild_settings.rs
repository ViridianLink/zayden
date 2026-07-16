use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::discord::{list_guild_channels, list_guild_roles};
use crate::server::guild::{
    SaveChannelSettings,
    SaveLfgSettings,
    SaveRoleSettings,
    SaveSupportSettings,
    SaveTempVoiceSettings,
    get_guild_settings,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

const TEXT_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
];

fn sel(value: Option<&str>) -> String {
    value.unwrap_or_default().to_owned()
}

#[component]
pub(crate) fn GuildSettingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let data = Resource::new_blocking(guild_id, |gid| async move {
        let settings = get_guild_settings(gid.clone()).await?;
        let channels = list_guild_channels(gid.clone()).await.unwrap_or_default();
        let roles = list_guild_roles(gid).await.unwrap_or_default();
        Ok::<(GuildSettings, Vec<ChannelInfo>, Vec<RoleInfo>), ServerFnError>((
            settings, channels, roles,
        ))
    });

    let save_support = ServerAction::<SaveSupportSettings>::new();
    let save_channels = ServerAction::<SaveChannelSettings>::new();
    let save_roles = ServerAction::<SaveRoleSettings>::new();
    let save_temp_voice = ServerAction::<SaveTempVoiceSettings>::new();
    let save_lfg = ServerAction::<SaveLfgSettings>::new();

    view! {
        <Title text="Settings - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Server Settings"</h1>
                        <p class="page-lead">
                            "Point Zayden's features at the right channels and roles."
                        </p>
                    </div>
                </div>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading settings\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load settings: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((s, channels, roles)) => {
                            view! {
                                // Support
                                {let r = save_support.value();
                                let channels = channels.clone();
                                let roles = roles.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="message"/>"Support"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_support>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <ChannelSelect
                                                label="Support Channel"
                                                name="support_channel_id"
                                                selected=sel(s.support_channel_id.as_deref())
                                                channels=channels.clone()
                                                kinds=TEXT_KINDS
                                            />
                                            <RoleSelect
                                                label="Support Role"
                                                name="support_role_id"
                                                selected=sel(s.support_role_id.as_deref())
                                                roles=roles.clone()
                                            />
                                            <ChannelSelect
                                                label="FAQ Channel"
                                                name="faq_channel_id"
                                                selected=sel(s.faq_channel_id.as_deref())
                                                channels=channels.clone()
                                                kinds=TEXT_KINDS
                                            />
                                            <ChannelSelect
                                                label="Suggestions Channel"
                                                name="suggestions_channel_id"
                                                selected=sel(s.suggestions_channel_id.as_deref())
                                                channels=channels.clone()
                                                kinds=TEXT_KINDS
                                            />
                                            <ChannelSelect
                                                label="Review Channel"
                                                name="review_channel_id"
                                                selected=sel(s.review_channel_id.as_deref())
                                                channels=channels.clone()
                                                kinds=TEXT_KINDS
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Channels
                                {let r = save_channels.value();
                                let channels = channels.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="grid"/>"Channels"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_channels>
                                            <input type="hidden" name="guild" value=guild_id()/>
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
                                                channels=channels.clone()
                                                kinds=TEXT_KINDS
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Roles
                                {let r = save_roles.value();
                                let roles = roles.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="users"/>"Roles"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_roles>
                                            <input type="hidden" name="guild" value=guild_id()/>
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
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Temp Voice
                                {let r = save_temp_voice.value();
                                let channels = channels.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="music"/>"Temp Voice"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_temp_voice>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <ChannelSelect
                                                label="Category"
                                                name="temp_voice_category"
                                                selected=sel(s.temp_voice_category.as_deref())
                                                channels=channels.clone()
                                                kinds=&[ChannelType::GuildCategory]
                                            />
                                            <ChannelSelect
                                                label="Creator Channel"
                                                name="temp_voice_creator_channel"
                                                selected=sel(s.temp_voice_creator_channel.as_deref())
                                                channels=channels.clone()
                                                kinds=&[ChannelType::GuildVoice]
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // LFG (final block — moves the channel/role lists).
                                {let r = save_lfg.value();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="gamepad"/>"LFG"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_lfg>
                                            <input type="hidden" name="guild" value=guild_id()/>
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
                                }}
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
