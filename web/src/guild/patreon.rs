use patreon::{PatreonAnnounceRow, PatreonConnection};
use topcoat::context::Cx;
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::dto::PatreonStatus;
use super::error::{GuildError, server_err};
use super::form::{GuildForm, form_args};
use super::parse::parse_flag;
use crate::auth::{
    admin_guild_id,
    ensure_bot_can_post,
    ensure_guild_channel,
    web_state,
};

form_args! {
    PatreonSettingsForm { guild, channel_id, public_only }
}

/// The guild's Patreon connection and announcement settings.
pub(crate) async fn fetch_patreon_status(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<PatreonStatus, GuildError> {
    let connection =
        PatreonConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    let announce =
        PatreonAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;

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

/// The Patreon status of a guild the signed-in user manages. Nothing calls it;
/// it logs when something does.
pub async fn get_patreon_status(
    cx: &Cx,
    guild: &str,
) -> Result<PatreonStatus, GuildError> {
    tracing::warn!(guild, "get_patreon_status called; it has no known caller");
    let (guild_id, app) = admin_app(cx, guild).await?;
    fetch_patreon_status(app, guild_id).await
}

/// Sets where Patreon posts are announced. A blank channel stops
/// announcements and keeps the connection.
pub async fn save_patreon_settings(
    cx: &Cx,
    form: &PatreonSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.channel_id.trim();

    // Clearing the channel is how a guild stops announcements without giving
    // up the connection, so an empty submission deletes the row.
    if channel_id.is_empty() {
        PatreonAnnounceRow::delete(&app.db, guild_id)
            .await
            .map(|_removed| ())
            .map_err(server_err)?;

        return Ok(());
    }

    let Ok(channel_id) = channel_id.parse::<i64>() else {
        return Err(GuildError::InvalidChannelId);
    };

    ensure_guild_channel(cx, guild_id, channel_id).await?;
    ensure_bot_can_post(cx, guild_id, channel_id).await?;

    PatreonAnnounceRow::upsert(
        &app.db,
        guild_id,
        channel_id,
        parse_flag(&form.public_only),
    )
    .await
    .map_err(server_err)
}

/// Whether the signed-in user may manage `guild`'s Patreon connection.
/// Nothing calls it; it logs when something does.
pub async fn can_manage_patreon(cx: &Cx, guild: &str) -> Result<bool, GuildError> {
    tracing::warn!(guild, "can_manage_patreon called; it has no known caller");
    admin_guild_id(cx, guild).await.map(|_id| true).map_err(GuildError::from)
}

/// Drops the guild's Patreon connection, then releases its webhook and
/// campaign.
pub async fn disconnect_patreon(
    cx: &Cx,
    form: &GuildForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let connection =
        PatreonConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    PatreonConnection::delete(&app.db, guild_id).await.map_err(server_err)?;

    if let Some(connection) = connection {
        let patreon_app =
            web_state(cx).ok().and_then(|state| state.integrations.patreon.as_ref());
        patreon::release(&app.http, &app.db, patreon_app, &connection).await;
    }

    Ok(())
}
