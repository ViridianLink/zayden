use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, ReactionRoleInfo, RoleInfo};
use crate::server::discord::{list_guild_channels, list_guild_roles};
use crate::server::reaction_roles::{
    AddReactionRole,
    RemoveReactionRole,
    list_reaction_roles,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;
use crate::ui::components::select::{ChannelSelect, RoleSelect};
use crate::ui::components::settings::{SettingField, save_feedback};

const TEXT_KINDS: &[ChannelType] =
    &[ChannelType::GuildText, ChannelType::GuildAnnouncement];

fn custom_emoji_id(emoji: &str) -> Option<&str> {
    let inner = emoji.strip_prefix('<')?.strip_suffix('>')?;
    let id = inner.rsplit(':').next()?;
    id.chars().all(|c| c.is_ascii_digit()).then_some(id)
}

fn emoji_view(emoji: &str) -> AnyView {
    custom_emoji_id(emoji).map_or_else(
        || view! { <span class="rr-emoji">{emoji.to_owned()}</span> }.into_any(),
        |id| {
            let src = format!("https://cdn.discordapp.com/emojis/{id}.png?size=32");
            view! { <img class="rr-emoji-img" src=src alt=""/> }.into_any()
        },
    )
}

#[component]
pub(crate) fn ReactionRolesPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let add = ServerAction::<AddReactionRole>::new();
    let remove = ServerAction::<RemoveReactionRole>::new();

    let data = Resource::new_blocking(
        move || (guild_id(), add.version().get(), remove.version().get()),
        |(gid, ..)| async move {
            let maps = list_reaction_roles(gid.clone()).await?;
            let channels =
                list_guild_channels(gid.clone()).await.unwrap_or_default();
            let roles = list_guild_roles(gid).await.unwrap_or_default();
            Ok::<
                (Vec<ReactionRoleInfo>, Vec<ChannelInfo>, Vec<RoleInfo>),
                ServerFnError,
            >((maps, channels, roles))
        },
    );

    let add_result = add.value();
    let remove_result = remove.value();

    view! {
        <Title text="Reaction Roles - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Reaction Roles"</h1>
                        <p class="page-lead">
                            "Every message \u{2192} emoji \u{2192} role mapping in this "
                            "server, in one place. Members react to get the role and "
                            "un-react to lose it."
                        </p>
                    </div>
                </div>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading reaction roles\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load reaction roles: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((maps, channels, roles)) => {
                            let gid = guild_id();
                            let form_channels = channels.clone();
                            let form_roles = roles.clone();
                            view! {
                                {move || remove_result.get().map(save_feedback)}
                                <MappingTable
                                    guild_id=gid.clone()
                                    maps=maps
                                    channels=channels
                                    roles=roles
                                    remove=remove
                                />

                                <fieldset class="settings-section">
                                    <legend><Icon name="plus"/>"Add a mapping"</legend>
                                    {move || add_result.get().map(save_feedback)}
                                    <ActionForm action=add>
                                        <input type="hidden" name="guild" value=gid.clone()/>
                                        <ChannelSelect
                                            label="Channel"
                                            name="channel_id"
                                            selected=String::new()
                                            channels=form_channels
                                            kinds=TEXT_KINDS
                                        />
                                        <SettingField
                                            label="Message ID (blank posts a new panel)"
                                            name="message_id"
                                            value=String::new()
                                        />
                                        <div class="setting-field">
                                            <label>"Emoji"</label>
                                            <input
                                                type="text"
                                                name="emoji"
                                                placeholder="\u{2705} or <:name:id>"
                                            />
                                        </div>
                                        <RoleSelect
                                            label="Role"
                                            name="role_id"
                                            selected=String::new()
                                            roles=form_roles
                                        />
                                        <div class="form-actions">
                                            <button type="submit" class="btn btn-primary">
                                                "Add mapping"
                                            </button>
                                        </div>
                                    </ActionForm>
                                    <p class="page-lead">
                                        "Leave the message ID blank and Zayden posts a new "
                                        "panel message in the chosen channel. Give an ID to "
                                        "attach the mapping to a message that already exists "
                                        "- several emoji can share one message."
                                    </p>
                                </fieldset>
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}

#[component]
fn MappingTable(
    guild_id: String,
    maps: Vec<ReactionRoleInfo>,
    channels: Vec<ChannelInfo>,
    roles: Vec<RoleInfo>,
    remove: ServerAction<RemoveReactionRole>,
) -> impl IntoView {
    if maps.is_empty() {
        return view! {
            <div class="empty">
                "No reaction roles yet - add one below and Zayden will seed the "
                "reaction for members to click."
            </div>
        }
        .into_any();
    }

    let rows = maps
        .into_iter()
        .map(|m| {
            let channel = channels
                .iter()
                .find(|c| c.id == m.channel_id)
                .map_or_else(
                    || format!("#unknown ({})", m.channel_id),
                    |c| format!("#{}", c.name),
                );
            let role = roles.iter().find(|r| r.id == m.role_id).map_or_else(
                || format!("@unknown ({})", m.role_id),
                |r| format!("@{}", r.name),
            );
            let link = format!(
                "https://discord.com/channels/{guild_id}/{}/{}",
                m.channel_id, m.message_id
            );
            let gid = guild_id.clone();

            view! {
                <div class="rr-row">
                    <span class="rr-channel">{channel}</span>
                    <span class="rr-cell">{emoji_view(&m.emoji)}</span>
                    <span class="rr-role">{role}</span>
                    <a class="rr-link" href=link rel="external noreferrer" target="_blank">
                        "Message"
                        <Icon name="external-link"/>
                    </a>
                    <ActionForm action=remove attr:class="rr-remove">
                        <input type="hidden" name="guild" value=gid/>
                        <input type="hidden" name="channel_id" value=m.channel_id/>
                        <input type="hidden" name="message_id" value=m.message_id/>
                        <input type="hidden" name="emoji" value=m.emoji/>
                        <button type="submit" class="btn btn-ghost">"Remove"</button>
                    </ActionForm>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="rr-table">
            <div class="rr-row rr-head">
                <span>"Channel"</span>
                <span>"Emoji"</span>
                <span>"Role"</span>
                <span></span>
                <span></span>
            </div>
            {rows}
        </div>
    }
    .into_any()
}
