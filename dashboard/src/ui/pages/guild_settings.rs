use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::discord::{list_guild_channels, list_guild_roles};
use crate::server::guild::{
    AddSupportRole,
    CreateTempVoiceCreatorChannel,
    RemoveSupportRole,
    SaveAiSettings,
    SaveChannelSettings,
    SaveFamilySettings,
    SaveHoneypotSettings,
    SaveLfgSettings,
    SaveMusicSettings,
    SaveRoleSettings,
    SaveSupportSettings,
    SaveTempVoiceSettings,
    get_guild_settings,
    list_support_roles,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{
    SaveButton,
    SettingField,
    ToggleField,
    create_feedback,
    save_feedback,
};

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

    let create_creator = ServerAction::<CreateTempVoiceCreatorChannel>::new();
    let add_support_role = ServerAction::<AddSupportRole>::new();
    let remove_support_role = ServerAction::<RemoveSupportRole>::new();

    let data = Resource::new_blocking(
        move || {
            (
                guild_id(),
                create_creator.version().get(),
                add_support_role.version().get(),
                remove_support_role.version().get(),
            )
        },
        |(gid, ..)| async move {
            let settings = get_guild_settings(gid.clone()).await?;
            let support_roles =
                list_support_roles(gid.clone()).await.unwrap_or_default();
            let channels =
                list_guild_channels(gid.clone()).await.unwrap_or_default();
            let roles = list_guild_roles(gid).await.unwrap_or_default();
            Ok::<
                (GuildSettings, Vec<String>, Vec<ChannelInfo>, Vec<RoleInfo>),
                ServerFnError,
            >((settings, support_roles, channels, roles))
        },
    );

    let save_support = ServerAction::<SaveSupportSettings>::new();
    let save_channels = ServerAction::<SaveChannelSettings>::new();
    let save_roles = ServerAction::<SaveRoleSettings>::new();
    let save_temp_voice = ServerAction::<SaveTempVoiceSettings>::new();
    let save_family = ServerAction::<SaveFamilySettings>::new();
    let save_music = ServerAction::<SaveMusicSettings>::new();
    let save_honeypot = ServerAction::<SaveHoneypotSettings>::new();
    let save_ai = ServerAction::<SaveAiSettings>::new();
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
                        Ok((s, support_roles, channels, roles)) => {
                            view! {
                                // Support
                                {let r = save_support.value();
                                let channels = channels.clone();
                                let support_role_views = view! {
                                    <SupportRoleField
                                        guild_id=guild_id()
                                        support_roles=support_roles
                                        roles=roles.clone()
                                        add=add_support_role
                                        remove=remove_support_role
                                    />
                                };
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
                                            <SettingField
                                                label="Promote at net upvotes"
                                                name="promote_threshold"
                                                value=s.suggestions_promote_threshold.clone()
                                            />
                                            <SettingField
                                                label="Demote at or below"
                                                name="demote_threshold"
                                                value=s.suggestions_demote_threshold.clone()
                                                pattern="-?[0-9]*"
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                        <p class="page-lead">
                                            "A suggestion is posted to the review channel once "
                                            "its \u{1F44D} minus \u{1F44E} count reaches the promote "
                                            "threshold, and removed again if it falls to or below "
                                            "the demote threshold. Tune both to your server size "
                                            "- demote must stay below promote."
                                        </p>
                                        {support_role_views}
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
                                            <RoleSelect
                                                label="Verified Role"
                                                name="verified_role_id"
                                                selected=sel(s.verified_role_id.as_deref())
                                                roles=roles.clone()
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Temp Voice
                                {let r = save_temp_voice.value();
                                let c = create_creator.value();
                                let save_channels = channels.clone();
                                let create_channels = channels.clone();
                                let save_category = sel(s.temp_voice_category.as_deref());
                                let create_category = save_category.clone();
                                let creator = sel(s.temp_voice_creator_channel.as_deref());
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="music"/>"Temp Voice"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_temp_voice>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <ChannelSelect
                                                label="Category"
                                                name="temp_voice_category"
                                                selected=save_category
                                                channels=save_channels.clone()
                                                kinds=&[ChannelType::GuildCategory]
                                            />
                                            <ChannelSelect
                                                label="Creator Channel"
                                                name="temp_voice_creator_channel"
                                                selected=creator
                                                channels=save_channels
                                                kinds=&[ChannelType::GuildVoice]
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                        <p class="page-lead">
                                            "No creator channel yet? Zayden can make one for you "
                                            "and point the settings above at it."
                                        </p>
                                        {move || c.get().map(create_feedback)}
                                        <ActionForm action=create_creator>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <ChannelSelect
                                                label="Create Creator Channel In"
                                                name="temp_voice_category"
                                                selected=create_category
                                                channels=create_channels
                                                kinds=&[ChannelType::GuildCategory]
                                            />
                                            <div class="form-actions">
                                                <button type="submit" class="btn btn-secondary">
                                                    "Create Creator Channel"
                                                </button>
                                            </div>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Family
                                {let r = save_family.value();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="heart"/>"Family"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_family>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <SettingField
                                                label="Max Partners"
                                                name="max_partners"
                                                value=s.family_max_partners.clone()
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                    </fieldset>
                                }}

                                // Music — admin setup only.
                                {let r = save_music.value();
                                let roles = roles.clone();
                                let channels = channels.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="music"/>"Music"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_music>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <RoleSelect
                                                label="DJ Role"
                                                name="dj_role_id"
                                                selected=sel(s.music_dj_role_id.as_deref())
                                                roles=roles
                                            />
                                            <SettingField
                                                label="Auto-disconnect (seconds)"
                                                name="auto_disconnect_secs"
                                                value=s.music_auto_disconnect_secs.clone()
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
                                }}

                                // Honeypot — anti-spam trap.
                                {let r = save_honeypot.value();
                                let roles = roles.clone();
                                let channels = channels.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="shield"/>"Honeypot"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_honeypot>
                                            <input type="hidden" name="guild" value=guild_id()/>
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
                                }}

                                // AI chat — mention-trigged replies.
                                {let r = save_ai.value();
                                let channels = channels.clone();
                                view! {
                                    <fieldset class="settings-section">
                                        <legend><Icon name="sparkles"/>"AI Chat"</legend>
                                        {move || r.get().map(save_feedback)}
                                        <ActionForm action=save_ai>
                                            <input type="hidden" name="guild" value=guild_id()/>
                                            <ToggleField
                                                label="AI Responses"
                                                name="enabled"
                                                value=s.ai_enabled
                                            />
                                            <ChannelSelect
                                                label="Restrict to Channel"
                                                name="channel_id"
                                                selected=sel(s.ai_channel_id.as_deref())
                                                channels=channels
                                                kinds=TEXT_KINDS
                                            />
                                            <SaveButton/>
                                        </ActionForm>
                                        <p class="page-lead">
                                            "With AI responses on, Zayden replies in character "
                                            "whenever someone mentions him. Leave the channel "
                                            "unset to let him answer anywhere he can see, or "
                                            "pick one to keep him to a single room."
                                        </p>
                                        <p class="page-lead">
                                            "Every reply costs a model call, so scope this to a "
                                            "channel you actually want him talking in. The "
                                            "toggle here is the same switch as the AI Chat card "
                                            "on the Modules page."
                                        </p>
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

#[component]
fn SupportRoleField(
    guild_id: String,
    support_roles: Vec<String>,
    roles: Vec<RoleInfo>,
    add: ServerAction<AddSupportRole>,
    remove: ServerAction<RemoveSupportRole>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();

    let unconfigured = roles
        .iter()
        .filter(|r| !support_roles.contains(&r.id))
        .cloned()
        .collect::<Vec<_>>();

    let chips = support_roles
        .into_iter()
        .map(|id| {
            let name = roles.iter().find(|r| r.id == id).map_or_else(
                || format!("@unknown ({id})"),
                |r| format!("@{}", r.name),
            );
            let gid = guild_id.clone();

            view! {
                <ActionForm action=remove attr:class="chip">
                    <input type="hidden" name="guild" value=gid/>
                    <input type="hidden" name="role_id" value=id/>
                    <span class="chip-label">{name}</span>
                    <button type="submit" class="chip-remove" title="Remove">
                        <Icon name="x"/>
                    </button>
                </ActionForm>
            }
        })
        .collect_view();

    view! {
        <div class="setting-field">
            <label>"Support Roles"</label>
            <p class="page-lead">
                "Pinged in every new ticket thread. With none set, Zayden falls "
                "back to pinging the server owner."
            </p>
            <div class="chip-list">{chips}</div>
            {move || remove_result.get().map(save_feedback)}
            {move || add_result.get().map(save_feedback)}
            <ActionForm action=add attr:class="chip-add">
                <input type="hidden" name="guild" value=guild_id/>
                <RoleSelect
                    label="Add a support role"
                    name="role_id"
                    selected=String::new()
                    roles=unconfigured
                />
                <button type="submit" class="btn btn-ghost">"Add role"</button>
            </ActionForm>
        </div>
    }
}
