use std::cmp::Reverse;
use std::sync::Arc;

use topcoat::context::Cx;
use twilight_http::Client;
use twilight_model::channel::permission_overwrite::PermissionOverwrite;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;

use super::context::discord_client;
use super::dto::{ChannelInfo, ForumTagInfo, RoleInfo};
use super::error::{AuthError, ForeignIdError};
use super::guild::admin_guild_id;
use super::ownership::GuildIds;

/// Permissions the bot needs to announce in a channel.
pub const ANNOUNCE_PERMISSIONS: Permissions =
    Permissions::VIEW_CHANNEL.union(Permissions::SEND_MESSAGES);

#[must_use]
pub fn bearer_client(access_token: &str) -> Client {
    Client::builder().token(format!("Bearer {access_token}")).build()
}

async fn admin_discord<'a>(
    cx: &'a Cx,
    guild: &str,
) -> Result<(u64, &'a Arc<Client>), AuthError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    Ok((guild_id.cast_unsigned(), discord_client(cx)?))
}

/// The guild's channels in display order, for a user who may manage it.
pub async fn list_guild_channels(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<ChannelInfo>, AuthError> {
    let (guild_id, http) = admin_discord(cx, guild).await?;
    fetch_guild_channels(http, guild_id).await
}

/// The guild's roles, highest first and without `@everyone`, for a user who
/// may manage it.
pub async fn list_guild_roles(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<RoleInfo>, AuthError> {
    let (guild_id, http) = admin_discord(cx, guild).await?;
    fetch_guild_roles(http, guild_id).await
}

pub(crate) async fn fetch_guild_channels(
    http: &Client,
    guild_id: u64,
) -> Result<Vec<ChannelInfo>, AuthError> {
    let guild = Id::new_checked(guild_id).ok_or(AuthError::InvalidGuildId)?;
    let mut channels = http.guild_channels(guild).await?.model().await?;
    channels.sort_by_key(|c| c.position.unwrap_or_default());

    Ok(channels
        .into_iter()
        .map(|c| ChannelInfo {
            id: c.id.to_string(),
            name: c.name.unwrap_or_default(),
            kind: c.kind,
            tags: c
                .available_tags
                .unwrap_or_default()
                .into_iter()
                .map(|t| ForumTagInfo { id: t.id.to_string(), name: t.name })
                .collect(),
        })
        .collect())
}

pub(crate) async fn fetch_guild_roles(
    http: &Client,
    guild_id: u64,
) -> Result<Vec<RoleInfo>, AuthError> {
    let guild = Id::new_checked(guild_id).ok_or(AuthError::InvalidGuildId)?;
    let mut roles = http.roles(guild).await?.model().await?;
    roles.sort_by_key(|r| Reverse(r.position));

    Ok(roles
        .into_iter()
        .filter(|r| r.id.get() != guild_id)
        .map(|r| RoleInfo {
            id: r.id.to_string(),
            name: r.name,
            color: r.colors.primary_color,
        })
        .collect())
}

/// Fails with [`ForeignIdError::Channel`] unless the guild lists the channel.
pub async fn ensure_guild_channel(
    cx: &Cx,
    guild_id: i64,
    channel_id: i64,
) -> Result<(), AuthError> {
    GuildIds::default().channel(Some(channel_id)).ensure_in(cx, guild_id).await
}

/// A member's permissions in a channel, resolved the way Discord does: the
/// `@everyone` and held roles' permissions, Administrator granting all, then
/// the `@everyone`, role and member overwrites in that order.
#[must_use]
pub fn channel_permissions(
    guild_id: u64,
    member_id: u64,
    member_roles: &[u64],
    roles: &[(u64, Permissions)],
    overwrites: &[PermissionOverwrite],
) -> Permissions {
    let held = |id: u64| id == guild_id || member_roles.contains(&id);

    let base = roles
        .iter()
        .filter(|(id, _)| held(*id))
        .fold(Permissions::empty(), |acc, (_, permissions)| acc | *permissions);

    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::all();
    }

    let apply = |permissions: Permissions, allow: Permissions, deny: Permissions| {
        (permissions - deny) | allow
    };

    let overwrite = |id: u64| overwrites.iter().find(|o| o.id.get() == id);

    let mut permissions = base;

    if let Some(everyone) = overwrite(guild_id) {
        permissions = apply(permissions, everyone.allow, everyone.deny);
    }

    let (allow, deny) = overwrites
        .iter()
        .filter(|o| o.id.get() != guild_id && member_roles.contains(&o.id.get()))
        .fold((Permissions::empty(), Permissions::empty()), |(allow, deny), o| {
            (allow | o.allow, deny | o.deny)
        });
    permissions = apply(permissions, allow, deny);

    if let Some(member) = overwrite(member_id) {
        permissions = apply(permissions, member.allow, member.deny);
    }

    permissions
}

/// Fails with [`AuthError::BotCannotPost`] unless the bot can view and send
/// messages in the channel.
pub async fn ensure_bot_can_post(
    cx: &Cx,
    guild_id: i64,
    channel_id: i64,
) -> Result<(), AuthError> {
    let http = discord_client(cx)?;
    let guild = Id::new_checked(guild_id.cast_unsigned())
        .ok_or(AuthError::InvalidGuildId)?;
    let channel_ref = Id::new_checked(channel_id.cast_unsigned())
        .ok_or(ForeignIdError::Channel)?;

    let bot = http.current_user().await?.model().await?;
    let member = http.guild_member(guild, bot.id).await?.model().await?;
    let roles = http.roles(guild).await?.model().await?;
    let channel = http.channel(channel_ref).await?.model().await?;

    let member_roles: Vec<u64> = member.roles.iter().map(|id| id.get()).collect();
    let roles: Vec<(u64, Permissions)> =
        roles.iter().map(|role| (role.id.get(), role.permissions)).collect();

    let granted = channel_permissions(
        guild.get(),
        bot.id.get(),
        &member_roles,
        &roles,
        channel.permission_overwrites.as_deref().unwrap_or_default(),
    );

    let missing = ANNOUNCE_PERMISSIONS - granted;
    if missing.is_empty() {
        return Ok(());
    }

    tracing::warn!(
        guild_id,
        channel_id,
        ?missing,
        "announcement channel rejected: the bot lacks permissions there"
    );

    Err(AuthError::BotCannotPost {
        bot: bot.name,
        channel: channel.name.unwrap_or_default(),
    })
}
