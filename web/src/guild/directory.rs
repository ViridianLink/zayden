use topcoat::context::Cx;
use twilight_model::id::Id;

use super::dto::{GuildDirectory, GuildInfo};
use super::error::GuildError;
use crate::auth::{
    AuthError,
    admin_guild_id,
    current_session_identity,
    discord_client,
    fetch_guild_channels,
    fetch_guild_roles,
    find_user_guild,
    manages_guild,
    user_guilds,
};
use crate::util::server_error_text;

/// The guilds the signed-in user manages, for the guild list and the server
/// switcher. Signed out is [`AuthError::Unauthenticated`].
pub async fn list_manageable_guilds(cx: &Cx) -> Result<Vec<GuildInfo>, GuildError> {
    let Some(identity) = current_session_identity(cx).await? else {
        return Err(AuthError::Unauthenticated.into());
    };

    let all_guilds = user_guilds(cx, &identity).await?;

    Ok(all_guilds
        .iter()
        .filter(|g| manages_guild(g))
        .map(|g| GuildInfo {
            id: g.id.to_string(),
            name: g.name.clone(),
            icon: g.icon.map(|hash| hash.to_string()),
        })
        .collect())
}

async fn user_guild_info(cx: &Cx, guild: &str, guild_id: u64) -> Option<GuildInfo> {
    let identity = current_session_identity(cx).await.ok()??;
    let guilds = user_guilds(cx, &identity).await.ok()?;

    find_user_guild(&guilds, guild_id).map(|g| GuildInfo {
        id: guild.to_owned(),
        name: g.name.clone(),
        icon: g.icon.map(|hash| hash.to_string()),
    })
}

async fn bot_guild_info(cx: &Cx, guild: &str, guild_id: u64) -> Option<GuildInfo> {
    let http = discord_client(cx).ok()?;
    let found =
        http.guild(Id::new_checked(guild_id)?).await.ok()?.model().await.ok()?;

    Some(GuildInfo {
        id: guild.to_owned(),
        name: found.name,
        icon: found.icon.map(|hash| hash.to_string()),
    })
}

/// The guild being managed, for the server switcher: from the user's own
/// guild list, else from the bot, else the bare id as its name.
pub async fn get_active_guild(
    cx: &Cx,
    guild: &str,
) -> Result<GuildInfo, GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?.cast_unsigned();

    // The authorization above already cached the viewer's own guild payload, so
    // only an operator viewing a guild they are not in pays a Discord call here.
    if let Some(info) = user_guild_info(cx, guild, guild_id).await {
        return Ok(info);
    }

    if let Some(info) = bot_guild_info(cx, guild, guild_id).await {
        return Ok(info);
    }

    Ok(GuildInfo { name: guild.to_owned(), id: guild.to_owned(), icon: None })
}

/// The guild's channels and roles for the settings pickers.
pub async fn get_guild_directory(
    cx: &Cx,
    guild: &str,
) -> Result<GuildDirectory, GuildError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let http = discord_client(cx)?;
    let discord_guild_id = guild_id.cast_unsigned();

    let (channels, roles) = tokio::join!(
        fetch_guild_channels(http, discord_guild_id),
        fetch_guild_roles(http, discord_guild_id),
    );

    Ok(GuildDirectory {
        channels: channels.map_err(server_error_text),
        roles: roles.map_err(server_error_text),
    })
}
