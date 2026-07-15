use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::server::guild::{
    SaveChannelSettings,
    SaveLfgSettings,
    SaveRoleSettings,
    SaveSupportSettings,
    SaveTempVoiceSettings,
    get_guild_settings,
};
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

#[component]
pub(crate) fn GuildSettingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let settings = Resource::new_blocking(guild_id, get_guild_settings);

    let save_support = ServerAction::<SaveSupportSettings>::new();
    let save_channels = ServerAction::<SaveChannelSettings>::new();
    let save_roles = ServerAction::<SaveRoleSettings>::new();
    let save_temp_voice = ServerAction::<SaveTempVoiceSettings>::new();
    let save_lfg = ServerAction::<SaveLfgSettings>::new();

    let modules_href = move || format!("/guild/{}/modules", guild_id());

    view! {
        <Title text="Settings - Zayden Dashboard"/>
        <div class="page">
            <h1>"Server Settings"</h1>
            <p class="page-lead">
                <A href=modules_href>"Manage enabled modules \u{2192}"</A>
            </p>
            <Suspense fallback=|| view! { <p class="loading">"Loading settings\u{2026}"</p> }>
                {move || settings.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="error">"Failed to load settings: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(s) => {
                        let is_pro = s.is_pro;
                        view! {
                            <Show when=move || !is_pro>
                                <div class="banner-pro">
                                    "This server is on the Free tier. "
                                    "Upgrade to Pro to save settings."
                                </div>
                            </Show>

                            // Support
                            {let r = save_support.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Support"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_support>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Support Channel ID"
                                            name="support_channel_id"
                                            value=s.support_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Support Role ID"
                                            name="support_role_id"
                                            value=s.support_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="FAQ Channel ID"
                                            name="faq_channel_id"
                                            value=s.faq_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Suggestions Channel ID"
                                            name="suggestions_channel_id"
                                            value=s.suggestions_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Review Channel ID"
                                            name="review_channel_id"
                                            value=s.review_channel_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            {let r = save_channels.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Channels"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_channels>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Rules Channel ID"
                                            name="rules_channel_id"
                                            value=s.rules_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="General Channel ID"
                                            name="general_channel_id"
                                            value=s.general_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Spoiler Channel ID"
                                            name="spoiler_channel_id"
                                            value=s.spoiler_channel_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            {let r = save_roles.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Roles"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_roles>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Artist Role ID"
                                            name="artist_role_id"
                                            value=s.artist_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Sleep Role ID"
                                            name="sleep_role_id"
                                            value=s.sleep_role_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // Temp Voice
                            {let r = save_temp_voice.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"Temp Voice"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_temp_voice>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="Category ID"
                                            name="temp_voice_category"
                                            value=s.temp_voice_category.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="Creator Channel ID"
                                            name="temp_voice_creator_channel"
                                            value=s.temp_voice_creator_channel.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}

                            // LFG
                            {let r = save_lfg.value();
                            view! {
                                <fieldset class="settings-section">
                                    <legend>"LFG"</legend>
                                    {move || r.get().map(save_feedback)}
                                    <ActionForm action=save_lfg>
                                        <input type="hidden" name="guild" value=guild_id()/>
                                        <SettingField
                                            label="LFG Channel ID"
                                            name="lfg_channel_id"
                                            value=s.lfg_channel_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="LFG Role ID"
                                            name="lfg_role_id"
                                            value=s.lfg_role_id.clone().unwrap_or_default()
                                        />
                                        <SettingField
                                            label="LFG Scheduled Thread ID"
                                            name="lfg_scheduled_thread_id"
                                            value=s.lfg_scheduled_thread_id.clone().unwrap_or_default()
                                        />
                                        <SaveButton is_pro=is_pro/>
                                    </ActionForm>
                                </fieldset>
                            }}
                        }.into_any()
                    },
                })}
            </Suspense>
        </div>
    }
}
