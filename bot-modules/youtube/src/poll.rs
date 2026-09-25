use reqwest::Client;
use sqlx::PgPool;
use tracing::{error, warn};

use crate::api;
use crate::error::{Result, YoutubeError};
use crate::store::{self, YoutubeChannelRow};

const FAILURE_ALERT: i32 = 5;

pub async fn poll_channel(
    client: &Client,
    pool: &PgPool,
    api_key: &str,
    channel: &YoutubeChannelRow,
) -> Result<usize> {
    let videos = api::fetch_uploads(
        client,
        api_key,
        &channel.uploads_playlist_id,
        &channel.channel_id,
    )
    .await?;

    let seeded = channel.is_seeded();
    let mut stored = 0_usize;

    for video in &videos {
        if store::insert_video(pool, video, !seeded).await? {
            stored += 1;
        }
    }

    YoutubeChannelRow::record_success(pool, &channel.channel_id).await?;

    Ok(stored)
}

pub async fn poll_by_id(
    client: &Client,
    pool: &PgPool,
    api_key: &str,
    channel_id: &str,
) -> Result<usize> {
    let Some(channel) = YoutubeChannelRow::select(pool, channel_id).await? else {
        return Ok(0);
    };

    match poll_channel(client, pool, api_key, &channel).await {
        Ok(stored) => Ok(stored),
        Err(e) => {
            on_failure(pool, channel_id, &e).await;
            Err(e)
        },
    }
}

pub async fn on_failure(pool: &PgPool, channel_id: &str, error: &YoutubeError) {
    let failures = match YoutubeChannelRow::record_failure(pool, channel_id).await {
        Ok(failures) => failures,
        Err(e) => {
            error!(error = ?e, channel_id, "youtube: failed to record poll failure");
            return;
        },
    };

    if failures >= FAILURE_ALERT {
        error!(
            error = ?error,
            channel_id,
            failures,
            "youtube: channel has failed to poll repeatedly"
        );
    } else {
        warn!(error = ?error, channel_id, failures, "youtube: channel poll failed");
    }
}
