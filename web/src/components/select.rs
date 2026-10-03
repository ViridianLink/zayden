#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};
use twilight_model::channel::ChannelType;

use super::icons::{Icon, icon};
use crate::auth::{ChannelInfo, ForumTagInfo, RoleInfo};

const CHANNELS_UNAVAILABLE: &str = "Couldn't reach Discord; the channel list is \
                                    unavailable. Saving keeps the current value.";

const ROLES_UNAVAILABLE: &str = "Couldn't reach Discord; the role list is \
                                 unavailable. Saving keeps the current value.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub kind: ChannelType,
    pub tags: Vec<ForumTag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForumTag {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub name: String,
}

impl From<ChannelInfo> for Channel {
    fn from(channel: ChannelInfo) -> Self {
        Self {
            id: channel.id,
            name: channel.name,
            kind: channel.kind,
            tags: channel.tags.into_iter().map(ForumTag::from).collect(),
        }
    }
}

impl From<&ChannelInfo> for Channel {
    fn from(channel: &ChannelInfo) -> Self {
        Self {
            id: channel.id.clone(),
            name: channel.name.clone(),
            kind: channel.kind,
            tags: channel.tags.iter().map(ForumTag::from).collect(),
        }
    }
}

impl From<ForumTagInfo> for ForumTag {
    fn from(tag: ForumTagInfo) -> Self {
        Self { id: tag.id, name: tag.name }
    }
}

impl From<&ForumTagInfo> for ForumTag {
    fn from(tag: &ForumTagInfo) -> Self {
        Self { id: tag.id.clone(), name: tag.name.clone() }
    }
}

impl From<RoleInfo> for Role {
    fn from(role: RoleInfo) -> Self {
        Self { id: role.id, name: role.name }
    }
}

impl From<&RoleInfo> for Role {
    fn from(role: &RoleInfo) -> Self {
        Self { id: role.id.clone(), name: role.name.clone() }
    }
}

#[component]
pub async fn select_field(
    label: &str,
    name: &str,
    selected: &str,
    options: Result<Vec<SelectOption>, String>,
) -> Result<impl View> {
    Ok(view! {
        match options {
            Ok(options) => picker(
                label: label,
                name: name,
                selected: selected,
                options: &options
            ),
            Err(reason) => locked_picker(
                label: label,
                name: name,
                selected: selected,
                reason: &reason
            ),
        }
    })
}

#[component]
async fn picker(
    label: &str,
    name: &str,
    selected: &str,
    options: &[SelectOption],
) -> Result<impl View> {
    let has_selected = !selected.is_empty();
    let known = options.iter().any(|option| option.value == selected);
    let fallback = (has_selected && !known).then(|| format!("Unknown ({selected})"));

    Ok(view! {
        <div class="setting-field">
            <label>(label)</label>
            <div class="select">
                <select class="input" name=(name)>
                    <option value="" selected=(!has_selected)>"(not set)"</option>
                    if let Some(text) = fallback {
                        <option value=(selected) selected="">(text)</option>
                    }
                    #[key(index)]
                    for (index, option) in options.iter().enumerate() {
                        <option
                            value=(option.value.as_str())
                            selected=(option.value == selected)
                        >
                            (option.label.as_str())
                        </option>
                    }
                </select>
                <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
            </div>
        </div>
    })
}

#[component]
async fn locked_picker(
    label: &str,
    name: &str,
    selected: &str,
    reason: &str,
) -> Result<impl View> {
    let current = if selected.is_empty() {
        "(not set)".to_owned()
    } else {
        format!("Unchanged ({selected})")
    };

    Ok(view! {
        <div class="setting-field">
            <label>(label)</label>
            <div class="select">
                <select class="input" disabled="">
                    <option selected="">(current)</option>
                </select>
                <span class="select-chevron">icon(name: Icon::ChevronDown)</span>
            </div>
            <input type="hidden" name=(name) value=(selected)>
            <p class="field-hint field-warning">(reason)</p>
        </div>
    })
}

fn unavailable(reason: &str, detail: &str) -> String {
    format!("{reason} ({detail})")
}

const fn channel_prefix(kind: ChannelType) -> &'static str {
    match kind {
        ChannelType::GuildVoice => "\u{1F50A} ",
        ChannelType::GuildCategory => "\u{25B8} ",
        ChannelType::GuildAnnouncement => "\u{1F4E2} ",
        ChannelType::GuildStageVoice => "\u{1F3A4} ",
        ChannelType::GuildForum => "\u{1F4AC} ",
        ChannelType::GuildText
        | ChannelType::Private
        | ChannelType::Group
        | ChannelType::AnnouncementThread
        | ChannelType::PublicThread
        | ChannelType::PrivateThread
        | ChannelType::GuildDirectory
        | ChannelType::GuildMedia
        | ChannelType::Unknown(_)
        | _ => "# ",
    }
}

pub fn channel_options(
    channels: Result<&[Channel], &str>,
    kinds: &[ChannelType],
) -> Result<Vec<SelectOption>, String> {
    channels
        .map(|channels| {
            channels
                .iter()
                .filter(|channel| kinds.is_empty() || kinds.contains(&channel.kind))
                .map(|channel| SelectOption {
                    value: channel.id.clone(),
                    label: format!(
                        "{}{}",
                        channel_prefix(channel.kind),
                        channel.name
                    ),
                })
                .collect()
        })
        .map_err(|detail| unavailable(CHANNELS_UNAVAILABLE, detail))
}

pub fn role_options(
    roles: Result<&[Role], &str>,
) -> Result<Vec<SelectOption>, String> {
    roles
        .map(|roles| {
            roles
                .iter()
                .map(|role| SelectOption {
                    value: role.id.clone(),
                    label: format!("@{}", role.name),
                })
                .collect()
        })
        .map_err(|detail| unavailable(ROLES_UNAVAILABLE, detail))
}

pub fn forum_tag_options(
    channels: Result<&[Channel], &str>,
) -> Result<Vec<SelectOption>, String> {
    channels
        .map(|channels| {
            channels
                .iter()
                .flat_map(|channel| {
                    channel.tags.iter().map(move |tag| SelectOption {
                        value: tag.id.clone(),
                        label: format!("#{} / {}", channel.name, tag.name),
                    })
                })
                .collect()
        })
        .map_err(|detail| unavailable(CHANNELS_UNAVAILABLE, detail))
}

#[component]
pub async fn channel_select(
    label: &str,
    name: &str,
    selected: &str,
    channels: Result<&[Channel], &str>,
    #[default] kinds: &[ChannelType],
) -> Result<impl View> {
    Ok(view! {
        select_field(
            label: label,
            name: name,
            selected: selected,
            options: channel_options(channels, kinds)
        )
    })
}

#[component]
pub async fn role_select(
    label: &str,
    name: &str,
    selected: &str,
    roles: Result<&[Role], &str>,
) -> Result<impl View> {
    Ok(view! {
        select_field(
            label: label,
            name: name,
            selected: selected,
            options: role_options(roles)
        )
    })
}

#[component]
pub async fn forum_tag_select(
    label: &str,
    name: &str,
    selected: &str,
    channels: Result<&[Channel], &str>,
) -> Result<impl View> {
    Ok(view! {
        select_field(
            label: label,
            name: name,
            selected: selected,
            options: forum_tag_options(channels)
        )
    })
}
