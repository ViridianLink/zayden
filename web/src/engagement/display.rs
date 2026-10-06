//! Text the engagement pages derive from their data.

use twilight_model::channel::ChannelType;

use crate::auth::{ChannelInfo, RoleInfo};

pub const TEXT_KINDS: &[ChannelType] =
    &[ChannelType::GuildText, ChannelType::GuildAnnouncement];

pub const GATE_KINDS: &[ChannelType] = &[
    ChannelType::GuildText,
    ChannelType::GuildAnnouncement,
    ChannelType::GuildForum,
    ChannelType::GuildCategory,
];

/// The id of a stored custom emoji (`<:name:id>` or `<a:name:id>`); `None` for
/// a Unicode emoji.
#[must_use]
pub fn custom_emoji_id(emoji: &str) -> Option<&str> {
    let inner = emoji.strip_prefix('<')?.strip_suffix('>')?;
    let id = inner.rsplit(':').next()?;
    (!id.is_empty() && id.chars().all(|c| c.is_ascii_digit())).then_some(id)
}

#[must_use]
pub fn emoji_image_url(id: &str) -> String {
    format!("https://cdn.discordapp.com/emojis/{id}.png?size=32")
}

#[must_use]
pub fn message_link(guild: &str, channel_id: &str, message_id: &str) -> String {
    format!("https://discord.com/channels/{guild}/{channel_id}/{message_id}")
}

#[must_use]
pub fn channel_label(channels: &[ChannelInfo], id: &str) -> String {
    channels
        .iter()
        .find(|c| c.id == id)
        .map_or_else(|| format!("#unknown ({id})"), |c| format!("#{}", c.name))
}

#[must_use]
pub fn role_label(roles: &[RoleInfo], id: &str) -> String {
    roles
        .iter()
        .find(|r| r.id == id)
        .map_or_else(|| format!("@unknown ({id})"), |r| format!("@{}", r.name))
}

#[must_use]
pub fn unconfigured_channels(
    channels: &[ChannelInfo],
    allowed: &[String],
) -> Vec<ChannelInfo> {
    channels.iter().filter(|c| !allowed.contains(&c.id)).cloned().collect()
}
