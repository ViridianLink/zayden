use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::admin_guild_id,
    crate::server::guild::admin_app,
    patreon::{PatreonAnnounceRow, PatreonConnection},
};

use crate::dto::PatreonStatus;

#[server]
pub async fn get_patreon_status(
    guild: String,
) -> Result<PatreonStatus, ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let connection = PatreonConnection::select(&app.db, guild_id)
        .await
        .map_err(crate::server::auth::server_err)?;

    let announce = PatreonAnnounceRow::select(&app.db, guild_id)
        .await
        .map_err(crate::server::auth::server_err)?;

    Ok(PatreonStatus {
        connected: connection.is_some(),
        disabled: connection.as_ref().is_some_and(|c| c.disabled_at.is_some()),
        creator_name: connection.as_ref().and_then(|c| c.creator_name.clone()),
        campaign_id: connection.as_ref().map(|c| c.campaign_id.clone()),
        webhook_registered: connection
            .as_ref()
            .is_some_and(|c| c.webhook_id.is_some()),
        channel_id: announce.as_ref().map(|a| a.channel_id.to_string()),
        public_only: announce.as_ref().is_some_and(|a| a.public_only),
    })
}

#[server]
pub async fn save_patreon_settings(
    guild: String,
    channel_id: String,
    public_only: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let channel_id = channel_id.trim();

    // Clearing the channel is how a guild stops announcements without giving
    // up the connection, so an empty submission deletes the row.
    if channel_id.is_empty() {
        PatreonAnnounceRow::delete(&app.db, guild_id)
            .await
            .map(|_removed| ())
            .map_err(crate::server::auth::server_err)?;

        return Ok(());
    }

    let Ok(channel_id) = channel_id.parse::<i64>() else {
        return Err(ServerFnError::ServerError("invalid channel id".to_string()));
    };

    PatreonAnnounceRow::upsert(
        &app.db,
        guild_id,
        channel_id,
        public_only.trim() == "true",
    )
    .await
    .map_err(crate::server::auth::server_err)
}

#[server]
pub async fn can_manage_patreon(guild: String) -> Result<bool, ServerFnError> {
    admin_guild_id(&guild).await.map(|_id| true)
}
