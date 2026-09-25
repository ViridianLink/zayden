use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::server_err,
    crate::server::discord::ensure_guild_channel,
    crate::server::guild::admin_app,
    youtube::websub::{self, Mode},
    youtube::{
        YoutubeAnnounceRow,
        YoutubeChannelRow,
        YoutubeConnection,
        YoutubeRuntime,
    },
    zayden_app::state::AppState,
};

#[cfg(feature = "ssr")]
use crate::dto::YoutubeStatus;

#[cfg(feature = "ssr")]
pub(crate) async fn fetch_youtube_status(
    app: &AppState,
    guild_id: i64,
) -> Result<YoutubeStatus, ServerFnError> {
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

#[server]
pub async fn save_youtube_settings(
    guild: String,
    channel_id: String,
) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let channel_id = channel_id.trim();

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
        return Err(ServerFnError::ServerError("invalid channel id".to_string()));
    };

    ensure_guild_channel(guild_id, channel_id).await?;

    YoutubeAnnounceRow::upsert(&app.db, guild_id, channel_id)
        .await
        .map_err(server_err)
}

#[server]
pub async fn disconnect_youtube(guild: String) -> Result<(), ServerFnError> {
    let (guild_id, app) = admin_app(&guild).await?;

    let Some(channel_id) =
        YoutubeConnection::delete(&app.db, guild_id).await.map_err(server_err)?
    else {
        return Ok(());
    };

    // Best effort: the hub subscription is shared by every guild on the
    // channel, so it only goes once the last of them has left.
    if let Some(runtime) = use_context::<YoutubeRuntime>() {
        release_subscription(&app, &runtime, &channel_id).await;
    }

    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn release_subscription(
    app: &AppState,
    runtime: &YoutubeRuntime,
    channel_id: &str,
) {
    if !matches!(
        YoutubeConnection::channel_has_connections(&app.db, channel_id).await,
        Ok(false)
    ) {
        return;
    }

    let Ok(Some(channel)) = YoutubeChannelRow::select(&app.db, channel_id).await
    else {
        return;
    };

    if let Err(e) = websub::request(
        &app.http,
        Mode::Unsubscribe,
        &runtime.webhook_uri,
        channel_id,
        &channel.websub_secret,
    )
    .await
    {
        tracing::warn!(?e, channel_id, "failed to unsubscribe from YouTube uploads");
    }

    if let Err(e) = YoutubeChannelRow::clear_lease(&app.db, channel_id).await {
        tracing::warn!(?e, channel_id, "failed to clear the YouTube lease");
    }
}
