use reqwest::Client;
use sqlx::PgPool;
use tracing::warn;

use crate::store::{YoutubeChannelRow, YoutubeConnection};
use crate::websub::{self, Mode};

pub async fn release_channel(
    client: &Client,
    pool: &PgPool,
    webhook_uri: Option<&str>,
    channel_id: &str,
) {
    if !matches!(
        YoutubeConnection::channel_has_connections(pool, channel_id).await,
        Ok(false)
    ) {
        return;
    }

    if let Some(webhook_uri) = webhook_uri {
        unsubscribe(client, pool, webhook_uri, channel_id).await;
    }

    if let Err(e) = YoutubeChannelRow::forget_videos(pool, channel_id).await {
        warn!(?e, channel_id, "failed to delete a released channel's videos");
    }
}

async fn unsubscribe(
    client: &Client,
    pool: &PgPool,
    webhook_uri: &str,
    channel_id: &str,
) {
    let Ok(Some(channel)) = YoutubeChannelRow::select(pool, channel_id).await else {
        return;
    };

    if let Err(e) = websub::request(
        client,
        Mode::Unsubscribe,
        webhook_uri,
        channel_id,
        &channel.websub_secret,
    )
    .await
    {
        warn!(?e, channel_id, "failed to unsubscribe from YouTube uploads");
    }

    if let Err(e) = YoutubeChannelRow::clear_lease(pool, channel_id).await {
        warn!(?e, channel_id, "failed to clear the YouTube lease");
    }
}
