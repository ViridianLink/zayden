pub mod ai;
pub mod family;
pub mod general;
pub mod honeypot;
pub mod lfg;
pub mod music;
pub mod support;
pub mod temp_voice;

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::components::A;
use leptos_router::hooks::{use_params_map, use_query_map};
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, GuildSettings, RoleInfo};
use crate::server::discord::{list_guild_channels, list_guild_roles};
use crate::server::guild::{
    AddSupportRole,
    CreateTempVoiceCreatorChannel,
    RemoveSupportRole,
    get_guild_settings,
    list_support_roles,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;

pub(super) const TEXT_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
];

pub(super) fn sel(value: Option<&str>) -> String {
    value.unwrap_or_default().to_owned()
}

struct Tab {
    slug: &'static str,
    label: &'static str,
    icon: &'static str,
    lead: &'static str,
}

const GENERAL: Tab = Tab {
    slug: "general",
    label: "General",
    icon: "grid",
    lead: "Server-wide channels and roles the rest of Zayden points at.",
};

const TABS: &[Tab] = &[
    GENERAL,
    Tab {
        slug: "support",
        label: "Support",
        icon: "ticket",
        lead: "Tickets, FAQ and suggestions - where they live and who gets pinged.",
    },
    Tab {
        slug: "temp-voice",
        label: "Temp Voice",
        icon: "mic",
        lead: "On-demand voice channels created from a join-to-create channel.",
    },
    Tab {
        slug: "music",
        label: "Music",
        icon: "music",
        lead: "Playback permissions and now-playing announcements.",
    },
    Tab {
        slug: "lfg",
        label: "LFG",
        icon: "gamepad",
        lead: "Where looking-for-group posts go and who they ping.",
    },
    Tab {
        slug: "family",
        label: "Family",
        icon: "heart",
        lead: "Limits for the family and relationship commands.",
    },
    Tab {
        slug: "ai",
        label: "AI Chat",
        icon: "sparkles",
        lead: "Whether Zayden answers when mentioned, and where.",
    },
    Tab {
        slug: "honeypot",
        label: "Honeypot",
        icon: "shield",
        lead: "The spam trap: a bait channel that bans whoever posts in it.",
    },
];

fn tab_def(slug: &str) -> &'static Tab {
    TABS.iter().find(|t| t.slug == slug).unwrap_or(&GENERAL)
}

#[component]
pub(crate) fn GuildSettingsPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let active = Memo::new(move |_| {
        query.with(|q| {
            q.get("tab")
                .and_then(|slug| TABS.iter().find(|t| t.slug == slug))
                .map_or(GENERAL.slug, |t| t.slug)
        })
    });

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

    view! {
        <Title text="Settings - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Server Settings"</h1>
                        <p class="page-lead">
                            {move || tab_def(active.get()).lead}
                        </p>
                    </div>
                </div>
                <SettingsTabs active=active/>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading settings\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load settings: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((s, support_roles, channels, roles)) => {
                            let gid = guild_id();
                            (move || {
                                let guild_id = gid.clone();
                                let s = s.clone();
                                let support_roles = support_roles.clone();
                                let channels = channels.clone();
                                let roles = roles.clone();

                                match active.get() {
                                    "support" => view! {
                                        <support::SupportTab
                                            guild_id=guild_id
                                            settings=s
                                            support_roles=support_roles
                                            channels=channels
                                            roles=roles
                                            add=add_support_role
                                            remove=remove_support_role
                                        />
                                    }.into_any(),
                                    "temp-voice" => view! {
                                        <temp_voice::TempVoiceTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            create=create_creator
                                        />
                                    }.into_any(),
                                    "music" => view! {
                                        <music::MusicTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                    "lfg" => view! {
                                        <lfg::LfgTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                            roles=roles
                                        />
                                    }.into_any(),
                                    "family" => view! {
                                        <family::FamilyTab guild_id=guild_id settings=s/>
                                    }.into_any(),
                                    "ai" => view! {
                                        <ai::AiTab
                                            guild_id=guild_id
                                            settings=s
                                            channels=channels
                                        />
                                    }.into_any(),
                                    "honeypot" => view! {
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

#[component]
fn SettingsTabs(active: Memo<&'static str>) -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let links = TABS
        .iter()
        .map(|tab| {
            let href =
                move || format!("/guild/{}/settings?tab={}", guild_id(), tab.slug);
            let class = move || {
                if active.get() == tab.slug {
                    "settings-tab active"
                } else {
                    "settings-tab"
                }
            };

            view! {
                <A
                    href=href
                    attr:class=class
                    attr:aria-current=move || {
                        (active.get() == tab.slug).then_some("page")
                    }
                >
                    <Icon name=tab.icon/>
                    <span>{tab.label}</span>
                </A>
            }
        })
        .collect_view();

    view! {
        <nav class="settings-tabs" aria-label="Settings sections">{links}</nav>
    }
}
