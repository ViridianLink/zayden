pub mod ai;
pub mod family;
pub mod general;
pub mod honeypot;
pub mod lfg;
pub mod music;
pub mod patreon;
pub mod support;
pub mod temp_voice;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use twilight_model::channel::ChannelType;

use crate::dto::{
    ChannelInfo,
    GuildSettings,
    HelperLinkInfo,
    PatreonStatus,
    RoleInfo,
};
use crate::server::discord::{list_guild_channels, list_guild_roles};
use crate::server::guild::{
    AddHelperLink,
    AddSupportRole,
    CreateTempVoiceCreatorChannel,
    RemoveHelperLink,
    RemoveSupportRole,
    get_guild_settings,
    list_helper_links,
    list_support_roles,
};
use crate::server::patreon::get_patreon_status;
use crate::ui::components::layout::AppShell;
use crate::ui::nav;

pub(super) const TEXT_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
];

pub(super) fn sel(value: Option<&str>) -> String {
    value.unwrap_or_default().to_owned()
}

#[component]
pub(crate) fn GuildSettingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    // Which module's panel to show. An unknown or missing section falls back to
    // General rather than rendering an empty page.
    let active = Memo::new(move |_| {
        let slug = params.with(|p| p.get("section").unwrap_or_default());
        nav::section(&slug)
    });

    let create_creator = ServerAction::<CreateTempVoiceCreatorChannel>::new();
    let add_support_role = ServerAction::<AddSupportRole>::new();
    let remove_support_role = ServerAction::<RemoveSupportRole>::new();
    let add_helper_link = ServerAction::<AddHelperLink>::new();
    let remove_helper_link = ServerAction::<RemoveHelperLink>::new();

    let data = Resource::new_blocking(
        move || {
            (
                guild_id(),
                create_creator.version().get(),
                add_support_role.version().get(),
                remove_support_role.version().get(),
                add_helper_link.version().get(),
                remove_helper_link.version().get(),
            )
        },
        |(gid, ..)| async move {
            let settings = get_guild_settings(gid.clone()).await?;
            let support_roles =
                list_support_roles(gid.clone()).await.unwrap_or_default();
            let helper_links =
                list_helper_links(gid.clone()).await.unwrap_or_default();
            let channels =
                list_guild_channels(gid.clone()).await.unwrap_or_default();
            let patreon = get_patreon_status(gid.clone()).await.unwrap_or_default();
            let roles = list_guild_roles(gid).await.unwrap_or_default();
            Ok::<
                (
                    GuildSettings,
                    Vec<String>,
                    Vec<HelperLinkInfo>,
                    Vec<ChannelInfo>,
                    Vec<RoleInfo>,
                    PatreonStatus,
                ),
                ServerFnError,
            >((
                settings,
                support_roles,
                helper_links,
                channels,
                roles,
                patreon,
            ))
        },
    );

    view! {
        <Title text=move || {
            format!("{} settings - Zayden Dashboard", active.get().label)
        }/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>{move || active.get().label}</h1>
                        <p class="page-lead">{move || active.get().lead()}</p>
                    </div>
                </div>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading settings\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load settings: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((s, support_roles, helper_links, channels, roles, patreon)) => {
                            let gid = guild_id();
                            // Re-runs on section change only; the resource above
                            // is untouched, so switching modules never refetches.
                            (move || {
                                let guild_id = gid.clone();
                                let s = s.clone();
                                let support_roles = support_roles.clone();
                                let helper_links = helper_links.clone();
                                let channels = channels.clone();
                                let roles = roles.clone();
                                let patreon_status = patreon.clone();

                                match active.get().slug() {
                                    Some("support") => view! {
                                        <support::SupportTab
                                            guild_id=guild_id
                                            settings=s
                                            support_roles=support_roles
                                            helper_links=helper_links
                                            channels=channels
                                            roles=roles
                                            add=add_support_role
                                            remove=remove_support_role
                                            add_link=add_helper_link
                                            remove_link=remove_helper_link
                                        />
                                    }.into_any(),
                                    Some("temp-voice") => view! {
                                        <temp_voice::TempVoiceTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            create=create_creator
                                        />
                                    }.into_any(),
                                    Some("music") => view! {
                                        <music::MusicTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                    Some("lfg") => view! {
                                        <lfg::LfgTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                    Some("family") => view! {
                                        <family::FamilyTab guild_id=guild_id settings=s/>
                                    }.into_any(),
                                    Some("ai") => view! {
                                        <ai::AiTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                        />
                                    }.into_any(),
                                    Some("patreon") => view! {
                                        <patreon::PatreonTab
                                            guild_id=guild_id
                                            status=patreon_status
                                            channels=channels
                                        />
                                    }.into_any(),
                                    Some("honeypot") => view! {
                                        <honeypot::HoneypotTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                    _ => view! {
                                        <general::GeneralTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                }
                            }).into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}
