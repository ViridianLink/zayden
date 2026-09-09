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

use crate::dto::{GuildDirectory, SectionSettings};
use crate::server::guild::{
    AddHelperLink,
    AddSupportRole,
    CreateTempVoiceCreatorChannel,
    RemoveHelperLink,
    RemoveSupportRole,
    get_guild_directory,
    get_section_settings,
};
use crate::server::patreon::DisconnectPatreon;
use crate::ui::components::skeleton::Skeleton;
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
    let disconnect_patreon = ServerAction::<DisconnectPatreon>::new();

    // Channel and role lists are the same on every tab, so they are keyed on
    // the guild alone and survive a section change untouched. Discord does not
    // cache them, and refetching per tab would be two API calls per click.
    let directory = Resource::new_blocking(guild_id, get_guild_directory);

    let settings = Resource::new_blocking(
        move || {
            (
                guild_id(),
                active.get().slug().unwrap_or("general").to_owned(),
                create_creator.version().get(),
                add_support_role.version().get(),
                remove_support_role.version().get(),
                add_helper_link.version().get(),
                remove_helper_link.version().get(),
                disconnect_patreon.version().get(),
            )
        },
        |(gid, section, ..)| async move { get_section_settings(gid, section).await },
    );

    view! {
        <Title text=move || {
            format!("{} settings - Zayden Dashboard", active.get().label)
        }/>
        <div class="page">
            <div class="page-header">
                <div>
                    <h1>{move || active.get().label}</h1>
                    <p class="page-lead">{move || active.get().lead()}</p>
                </div>
            </div>
            <Transition fallback=|| view! {
                <div class="skeleton-stack">
                    <Skeleton class="skeleton-panel" count=2/>
                </div>
            }>
                {move || directory.get().zip(settings.get()).map(|pair| {
                    let gid = guild_id();

                    match pair {
                        (Err(e), _) | (_, Err(e)) => view! {
                            <p class="error">
                                "Failed to load settings: " {e.to_string()}
                            </p>
                        }.into_any(),
                        (Ok(GuildDirectory { channels, roles }), Ok(section)) => {
                            match section {
                                SectionSettings::Support(s) => view! {
                                    <support::SupportTab
                                        guild_id=gid
                                        settings=*s
                                        channels=channels
                                        roles=roles
                                        add=add_support_role
                                        remove=remove_support_role
                                        add_link=add_helper_link
                                        remove_link=remove_helper_link
                                    />
                                }.into_any(),
                                SectionSettings::TempVoice(s) => view! {
                                    <temp_voice::TempVoiceTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                        create=create_creator
                                    />
                                }.into_any(),
                                SectionSettings::Music(s) => view! {
                                    <music::MusicTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                        roles=roles
                                    />
                                }.into_any(),
                                SectionSettings::Lfg(s) => view! {
                                    <lfg::LfgTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                        roles=roles
                                    />
                                }.into_any(),
                                SectionSettings::Family(s) => view! {
                                    <family::FamilyTab guild_id=gid settings=s/>
                                }.into_any(),
                                SectionSettings::Ai(s) => view! {
                                    <ai::AiTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                    />
                                }.into_any(),
                                SectionSettings::Patreon(status) => view! {
                                    <patreon::PatreonTab
                                        guild_id=gid
                                        status=status
                                        channels=channels
                                        disconnect=disconnect_patreon
                                    />
                                }.into_any(),
                                SectionSettings::Honeypot(s) => view! {
                                    <honeypot::HoneypotTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                        roles=roles
                                    />
                                }.into_any(),
                                SectionSettings::General(s) => view! {
                                    <general::GeneralTab
                                        guild_id=gid
                                        settings=s
                                        channels=channels
                                        roles=roles
                                    />
                                }.into_any(),
                            }
                        },
                    }
                })}
            </Transition>
        </div>
    }
}
