use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::dto::ForumTagInfo,
    crate::server::auth::{admin_guild_id, discord_client, server_err},
    crate::server::ownership::GuildIds,
    std::sync::Arc,
    twilight_http::Client,
    twilight_model::channel::permission_overwrite::PermissionOverwrite,
    twilight_model::guild::Permissions,
    twilight_model::id::Id,
};

use crate::dto::{ChannelInfo, RoleInfo};

#[cfg(feature = "ssr")]
async fn admin_discord(guild: &str) -> Result<(u64, Arc<Client>), ServerFnError> {
    let guild_id = admin_guild_id(guild).await?;
    Ok((guild_id.cast_unsigned(), discord_client()?))
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_guild_channels(
    http: &Client,
    guild_id: u64,
) -> Result<Vec<ChannelInfo>, ServerFnError> {
    let mut channels = http
        .guild_channels(Id::new(guild_id))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;
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

#[cfg(feature = "ssr")]
pub(crate) async fn ensure_guild_channel(
    guild_id: i64,
    channel_id: i64,
) -> Result<(), ServerFnError> {
    GuildIds::default().channel(Some(channel_id)).ensure_in(guild_id).await
}

#[cfg(feature = "ssr")]
pub const ANNOUNCE_PERMISSIONS: Permissions =
    Permissions::VIEW_CHANNEL.union(Permissions::SEND_MESSAGES);

#[cfg(feature = "ssr")]
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

#[cfg(feature = "ssr")]
pub(crate) async fn ensure_bot_can_post(
    guild_id: i64,
    channel_id: i64,
) -> Result<(), ServerFnError> {
    let http = discord_client()?;
    let guild = Id::new(guild_id.cast_unsigned());

    let bot = http
        .current_user()
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    let member = http
        .guild_member(guild, bot.id)
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    let roles = http
        .roles(guild)
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

    let channel = http
        .channel(Id::new(channel_id.cast_unsigned()))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;

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

    Err(ServerFnError::ServerError(format!(
        "{} can't post in #{}: it needs View Channel and Send Messages there. \
         Allow them for the bot's role in the channel's permissions, then save \
         again.",
        bot.name,
        channel.name.unwrap_or_default()
    )))
}

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_guild_roles(
    http: &Client,
    guild_id: u64,
) -> Result<Vec<RoleInfo>, ServerFnError> {
    let mut roles = http
        .roles(Id::new(guild_id))
        .await
        .map_err(server_err)?
        .model()
        .await
        .map_err(server_err)?;
    roles.sort_by_key(|r| std::cmp::Reverse(r.position));

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

#[server]
pub async fn list_guild_channels(
    guild: String,
) -> Result<Vec<ChannelInfo>, ServerFnError> {
    let (guild_id, http) = admin_discord(&guild).await?;
    fetch_guild_channels(&http, guild_id).await
}

#[server]
pub async fn list_guild_roles(
    guild: String,
) -> Result<Vec<RoleInfo>, ServerFnError> {
    let (guild_id, http) = admin_discord(&guild).await?;
    fetch_guild_roles(&http, guild_id).await
}
