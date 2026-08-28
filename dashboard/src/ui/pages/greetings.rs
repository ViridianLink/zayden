use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;
use twilight_model::channel::ChannelType;

use crate::dto::{ChannelInfo, CooldownView, GreetingImageInfo, GreetingsView};
use crate::server::discord::list_guild_channels;
use crate::server::greetings::{
    AddGreetingChannel,
    AddGreetingImage,
    RemoveGreetingChannel,
    RemoveGreetingImage,
    SaveGreetingCooldowns,
    SaveGreetingMessages,
    get_greetings,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;
use crate::ui::components::select::ChannelSelect;
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

const ANY_TEXT: &str = ".*";

const GATE_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
    ChannelType::GuildCategory,
];

#[component]
pub(crate) fn GreetingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let save = ServerAction::<SaveGreetingMessages>::new();
    let add = ServerAction::<AddGreetingImage>::new();
    let remove = ServerAction::<RemoveGreetingImage>::new();
    let save_cooldowns = ServerAction::<SaveGreetingCooldowns>::new();
    let add_channel = ServerAction::<AddGreetingChannel>::new();
    let remove_channel = ServerAction::<RemoveGreetingChannel>::new();

    let data = Resource::new_blocking(
        move || {
            (
                guild_id(),
                save.version().get(),
                add.version().get(),
                remove.version().get(),
                save_cooldowns.version().get(),
                add_channel.version().get(),
                remove_channel.version().get(),
            )
        },
        |(gid, ..)| async move {
            let view = get_greetings(gid.clone()).await?;
            let channels = list_guild_channels(gid).await.unwrap_or_default();
            Ok::<(GreetingsView, Vec<ChannelInfo>), ServerFnError>((view, channels))
        },
    );

    let save_result = save.value();
    let add_result = add.value();
    let remove_result = remove.value();

    view! {
        <Title text="Greetings - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Greetings"</h1>
                        <p class="page-lead">
                            "What Zayden posts for "<code>"/good morning"</code>" and "
                            <code>"/good night"</code>". Each subcommand replies with one "
                            "image picked at random from its list, plus the message "
                            "below if you set one."
                        </p>
                    </div>
                </div>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading greetings\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load greetings: " {e.to_string()}</p>
                        }.into_any(),
                        Ok((view, channels)) => {
                            let GreetingsView {
                                morning_message,
                                night_message,
                                morning,
                                night,
                                allowed_channels,
                                channels_locked,
                                cooldowns,
                            } = view;
                            let gid = guild_id();
                            let form_gid = gid.clone();
                            let morning_gid = gid.clone();
                            let channel_gid = gid.clone();
                            let cooldown_gid = gid.clone();

                            view! {
                                <fieldset class="settings-section">
                                    <legend><Icon name="message"/>"Messages"</legend>
                                    {move || save_result.get().map(save_feedback)}
                                    <ActionForm action=save>
                                        <input type="hidden" name="guild" value=form_gid/>
                                        <SettingField
                                            label="Good morning message"
                                            name="morning_message"
                                            value=morning_message
                                            pattern=ANY_TEXT
                                        />
                                        <SettingField
                                            label="Good night message"
                                            name="night_message"
                                            value=night_message
                                            pattern=ANY_TEXT
                                        />
                                        <PlaceholderLegend/>
                                        <SaveButton/>
                                    </ActionForm>
                                </fieldset>

                                <ChannelSection
                                    guild_id=channel_gid
                                    allowed=allowed_channels
                                    channels=channels
                                    locked=channels_locked
                                    add=add_channel
                                    remove=remove_channel
                                />

                                <CooldownSection
                                    guild_id=cooldown_gid
                                    cooldowns=cooldowns
                                    save=save_cooldowns
                                />

                                {move || add_result.get().map(save_feedback)}
                                {move || remove_result.get().map(save_feedback)}

                                <ImageSection
                                    guild_id=morning_gid
                                    kind="morning"
                                    title="Good morning images"
                                    images=morning
                                    add=add
                                    remove=remove
                                />
                                <ImageSection
                                    guild_id=gid
                                    kind="night"
                                    title="Good night images"
                                    images=night
                                    add=add
                                    remove=remove
                                />
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}

#[component]
fn PlaceholderLegend() -> impl IntoView {
    view! {
        <ul class="greet-legend">
            <li>
                <code>"{user}"</code>
                " - mentions the person being greeted, or the sender when "
                "the command is run without a user."
            </li>
            <li>
                <code>"{author}"</code>
                " - mentions whoever ran the command."
            </li>
            <li>"Leave a message blank to post just the image."</li>
        </ul>
    }
}

#[component]
fn ChannelSection(
    guild_id: String,
    allowed: Vec<String>,
    channels: Vec<ChannelInfo>,
    locked: bool,
    add: ServerAction<AddGreetingChannel>,
    remove: ServerAction<RemoveGreetingChannel>,
) -> impl IntoView {
    let add_result = add.value();
    let remove_result = remove.value();
    let add_gid = guild_id.clone();

    let unconfigured = channels
        .iter()
        .filter(|c| !allowed.contains(&c.id))
        .cloned()
        .collect::<Vec<_>>();

    let chips = allowed
        .into_iter()
        .map(|id| {
            let name = channels.iter().find(|c| c.id == id).map_or_else(
                || format!("#unknown ({id})"),
                |c| format!("#{}", c.name),
            );

            if locked {
                return view! {
                    <span class="chip"><span class="chip-label">{name}</span></span>
                }
                .into_any();
            }

            let gid = guild_id.clone();

            view! {
                <ActionForm action=remove attr:class="chip">
                    <input type="hidden" name="guild" value=gid/>
                    <input type="hidden" name="channel_id" value=id/>
                    <span class="chip-label">{name}</span>
                    <button type="submit" class="chip-remove" title="Remove">
                        <Icon name="x"/>
                    </button>
                </ActionForm>
            }
            .into_any()
        })
        .collect_view();

    let editor = if locked {
        view! {
            <p class="module-locked">
                "Read-only: Discord only lets a member with Manage Server \
                 change which channels a command is allowed in."
            </p>
        }
        .into_any()
    } else {
        view! {
            {move || remove_result.get().map(save_feedback)}
            {move || add_result.get().map(save_feedback)}
            <ActionForm action=add attr:class="chip-add">
                <input type="hidden" name="guild" value=add_gid/>
                <ChannelSelect
                    label="Allow a channel"
                    name="channel_id"
                    selected=String::new()
                    channels=unconfigured
                    kinds=GATE_KINDS
                />
                <button type="submit" class="btn btn-ghost">"Add channel"</button>
            </ActionForm>
        }
        .into_any()
    };

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="grid"/>"Where /good works"</legend>
            <p class="page-lead">
                "With nothing listed, "<code>"/good"</code>" works in every channel. "
                "Add one or more and Discord hides the command everywhere else "
                "- it never even shows up in the picker. Adding a category "
                "covers every channel inside it."
            </p>
            <p class="page-lead">
                "This writes the same command permissions as Discord's own "
                "Server Settings \u{2192} Integrations panel, so changes made "
                "either way show up in both."
            </p>
            <div class="chip-list">{chips}</div>
            {editor}
        </fieldset>
    }
}

#[component]
fn CooldownSection(
    guild_id: String,
    cooldowns: CooldownView,
    save: ServerAction<SaveGreetingCooldowns>,
) -> impl IntoView {
    let result = save.value();

    let user_label =
        format!("Per-member cooldown (seconds, min {})", cooldowns.floor_user_secs);
    let guild_label = format!(
        "Server-wide cooldown (seconds, min {})",
        cooldowns.floor_guild_secs
    );

    let upgrade = cooldowns.next_tier.map(|next| {
        let pitch = format!(
            "On {} these floors drop to {}s and {}s.",
            next.label(),
            cooldowns.next_floor_user_secs,
            cooldowns.next_floor_guild_secs,
        );

        view! {
            <p class="page-lead">
                {pitch}
                " "
                <a href="/upgrade">"See plans"</a>
                "."
            </p>
        }
    });

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="gauge"/>"Cooldowns"</legend>
            <p class="page-lead">
                "The per-member cooldown stops one person spamming "<code>"/good"</code>
                "; the server-wide one stops a crowd doing it between them. Both are "
                "in seconds, and both must stay at or above the minimum for this "
                "server's plan."
            </p>
            {move || result.get().map(save_feedback)}
            <ActionForm action=save>
                <input type="hidden" name="guild" value=guild_id/>
                <DynamicSettingField
                    label=user_label
                    name="user_cooldown"
                    value=cooldowns.user_secs.to_string()
                />
                <DynamicSettingField
                    label=guild_label
                    name="guild_cooldown"
                    value=cooldowns.guild_secs.to_string()
                />
                <SaveButton/>
            </ActionForm>
            {upgrade}
        </fieldset>
    }
}

#[component]
fn DynamicSettingField(
    label: String,
    name: &'static str,
    value: String,
) -> impl IntoView {
    view! {
        <div class="setting-field">
            <label>{label}</label>
            <input
                type="text"
                name=name
                value=value
                placeholder="(not set)"
                pattern="[0-9]*"
            />
        </div>
    }
}

#[component]
fn ImageSection(
    guild_id: String,
    kind: &'static str,
    title: &'static str,
    images: Vec<GreetingImageInfo>,
    add: ServerAction<AddGreetingImage>,
    remove: ServerAction<RemoveGreetingImage>,
) -> impl IntoView {
    let add_gid = guild_id.clone();

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="sparkles"/>{title}</legend>
            <ImageGrid guild_id=guild_id images=images remove=remove/>
            <ActionForm action=add>
                <input type="hidden" name="guild" value=add_gid/>
                <input type="hidden" name="kind" value=kind/>
                <div class="setting-field">
                    <label>"Image link"</label>
                    <input
                        type="url"
                        name="url"
                        placeholder="https://example.com/sunrise.gif"
                        pattern="https://.*"
                        required
                    />
                </div>
                <div class="form-actions">
                    <button type="submit" class="btn btn-primary">
                        <Icon name="plus"/>
                        "Add image"
                    </button>
                </div>
            </ActionForm>
            <p class="page-lead">
                "Links must start with "<code>"https://"</code>" and point straight "
                "at an image. Zayden embeds the link rather than storing a copy, so "
                "an image that later disappears from its host shows up blank here "
                "and in Discord. Up to 50 per greeting."
            </p>
        </fieldset>
    }
}

#[component]
fn ImageGrid(
    guild_id: String,
    images: Vec<GreetingImageInfo>,
    remove: ServerAction<RemoveGreetingImage>,
) -> impl IntoView {
    if images.is_empty() {
        return view! {
            <div class="empty">
                "No images yet - the command will reply with just the "
                "message until you add one."
            </div>
        }
        .into_any();
    }

    let cards = images
        .into_iter()
        .map(|image| {
            let gid = guild_id.clone();
            let src = image.url.clone();
            let href = image.url.clone();
            let title = image.url.clone();

            view! {
                <div class="greet-card">
                    <img class="greet-thumb" src=src alt="" loading="lazy"/>
                    <a
                        class="greet-url"
                        href=href
                        rel="external noreferrer"
                        target="_blank"
                        title=title
                    >
                        {image.url}
                    </a>
                    <ActionForm action=remove attr:class="greet-remove">
                        <input type="hidden" name="guild" value=gid/>
                        <input type="hidden" name="id" value=image.id/>
                        <button type="submit" class="btn btn-ghost">
                            <Icon name="x"/>
                            "Remove"
                        </button>
                    </ActionForm>
                </div>
            }
        })
        .collect_view();

    view! { <div class="greet-grid">{cards}</div> }.into_any()
}
