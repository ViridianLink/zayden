use std::time::Duration;

use futures::StreamExt;
use jiff::{SignedDuration, Timestamp};
use reqwest::Client;
use serenity::all::{CreateAllowedMentions, CreateMessage, Http, MessageBuilder};
use sqlx::PgPool;
use tracing::{debug, error};

use crate::error::Result;
use crate::model::video_url;
use crate::poll;
use crate::store::{self, PendingVideo, YoutubeAnnounceRow};

const CLAIM_BATCH: i64 = 20;
const CONCURRENCY: usize = 5;

pub const MAX_AGE: SignedDuration = SignedDuration::from_hours(48);
const PING_RETRY: Duration = Duration::from_secs(120);

pub async fn announce_pending(http: &Http, pool: &PgPool) -> Result<()> {
    let pending = store::claim_pending(pool, CLAIM_BATCH).await?;
    let cutoff = Timestamp::now() - MAX_AGE;

    for video in pending {
        if video.published_at.to_jiff() < cutoff {
            debug!(video_id = video.video_id, "youtube: skipping a stale video");
            continue;
        }

        let rows = match YoutubeAnnounceRow::for_channel(pool, &video.channel_id)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                error!(
                    error = ?e,
                    channel_id = video.channel_id,
                    "youtube: failed to load announce rows; the claimed item is dropped, not retried"
                );
                continue;
            },
        };

        broadcast(http, &video, rows).await;
    }

    Ok(())
}

pub async fn on_ping(
    http: &Http,
    client: &Client,
    pool: &PgPool,
    api_key: &str,
    channel_id: &str,
) {
    for attempt in 0..2 {
        if attempt > 0 {
            tokio::time::sleep(PING_RETRY).await;
        }

        match poll::poll_by_id(client, pool, api_key, channel_id).await {
            Ok(0) => {},
            Ok(_) => break,
            Err(_) => return,
        }
    }

    if let Err(e) = announce_pending(http, pool).await {
        error!(error = ?e, channel_id, "youtube: failed to announce a pinged upload");
    }
}

#[must_use]
pub fn message_content(video: &PendingVideo) -> String {
    MessageBuilder::new()
        .push("New video from ")
        .push_bold_safe(video.channel_title.as_str())
        .push("\n")
        .push(video_url(&video.video_id).as_str())
        .build()
}

async fn broadcast(
    http: &Http,
    video: &PendingVideo,
    rows: Vec<YoutubeAnnounceRow>,
) {
    let content = message_content(video);

    futures::stream::iter(rows.into_iter().map(|row| {
        let content = content.clone();
        async move {
            // The URL alone unfurls into Discord's playable YouTube embed.
            let message = CreateMessage::new()
                .content(content)
                .allowed_mentions(CreateAllowedMentions::new());

            if let Err(e) = row.channel().widen().send_message(http, message).await {
                error!(
                    error = ?e,
                    guild_id = row.guild_id,
                    video_id = video.video_id,
                    "youtube: failed to post announcement; it is not retried"
                );
            }
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .for_each(|()| async {})
    .await;
}
