use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{discord_client, guild_admin_context, server_err},
    std::sync::Arc,
    twilight_http::Client,
    twilight_model::id::Id,
};

use crate::dto::{ChannelInfo, RoleInfo};

#[cfg(feature = "ssr")]
async fn admin_discord(guild: &str) -> Result<(u64, Arc<Client>), ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(guild).await?;
    Ok((guild_id.cast_unsigned(), discord_client()?))
}

#[server]
pub async fn list_guild_channels(
    guild: String,
) -> Result<Vec<ChannelInfo>, ServerFnError> {
    let (guild_id, http) = admin_discord(&guild).await?;

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
        })
        .collect())
}

#[server]
pub async fn list_guild_roles(
    guild: String,
) -> Result<Vec<RoleInfo>, ServerFnError> {
    let (guild_id, http) = admin_discord(&guild).await?;

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
