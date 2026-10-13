use twilight_model::channel::ChannelType;

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
