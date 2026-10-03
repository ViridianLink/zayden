use topcoat::context::Cx;
use youtube::{YoutubeAnnounceRow, YoutubeConnection};
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::dto::YoutubeStatus;
use super::error::{GuildError, server_err};
use super::form::{GuildForm, form_args};
use crate::auth::{ensure_bot_can_post, ensure_guild_channel, web_state};

form_args! {
    YoutubeSettingsForm { guild, channel_id }
}

/// The guild's YouTube connection and announcement channel.
pub(crate) async fn fetch_youtube_status(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<YoutubeStatus, GuildError> {
    let connection =
        YoutubeConnection::select(&app.db, guild_id).await.map_err(server_err)?;

    let announce =
        YoutubeAnnounceRow::select(&app.db, guild_id).await.map_err(server_err)?;

    Ok(YoutubeStatus {
        connected: connection.is_some(),
        channel_title: connection.as_ref().map(|c| c.channel_title.clone()),
        push_active: connection.as_ref().is_some_and(|c| c.lease_active),
        channel_id: announce.as_ref().map(|a| a.channel_id.to_string()),
    })
}

/// Sets where YouTube uploads are announced. A blank channel stops
/// announcements and keeps the connection.
pub async fn save_youtube_settings(
    cx: &Cx,
    form: &YoutubeSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = form.channel_id.trim();

    // Clearing the channel stops announcements without giving up the
    // connection.
    if channel_id.is_empty() {
        YoutubeAnnounceRow::delete(&app.db, guild_id)
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

    YoutubeAnnounceRow::upsert(&app.db, guild_id, channel_id)
        .await
        .map_err(server_err)
}

/// Drops the guild's YouTube connection, then releases the channel if no
/// other guild still follows it.
pub async fn disconnect_youtube(
    cx: &Cx,
    form: &GuildForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let Some(channel_id) =
        YoutubeConnection::delete(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Ok(());
    };

    let runtime = web_state(cx)
        .ok()
        .and_then(|state| state.integrations.youtube_runtime.as_ref());
    youtube::release_channel(
        &app.http,
        &app.db,
        runtime.map(|r| &*r.webhook_uri),
        &channel_id,
    )
    .await;

    Ok(())
}
