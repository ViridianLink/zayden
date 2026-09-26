use futures::StreamExt;
use jiff::{SignedDuration, Timestamp};
use reqwest::Client;
use serenity::all::{CreateMessage, Http, MessageFlags};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::error::Result;
use crate::store::{self, PatreonAnnounceRow, PendingPost};
use crate::{embeds, thumbnail};

const CLAIM_BATCH: i64 = 20;
const CONCURRENCY: usize = 5;

pub const MAX_AGE: SignedDuration = SignedDuration::from_hours(48);

pub async fn announce_pending(
    http: &Http,
    client: &Client,
    pool: &PgPool,
) -> Result<()> {
    let pending = store::claim_pending(pool, CLAIM_BATCH).await?;

    if !pending.is_empty() {
        info!(count = pending.len(), "patreon: claimed posts to announce");
    }

    let cutoff = Timestamp::now() - MAX_AGE;

    for mut post in pending {
        if post.published_at.to_jiff() < cutoff {
            warn!(
                post_id = post.post_id,
                published_at = %post.published_at.to_jiff(),
                "patreon: post is older than the announce window; dropped"
            );
            continue;
        }

        if post.thumbnail_url.is_none()
            && let Some(url) = lookup_thumbnail(client, &post).await
        {
            if let Err(e) = store::set_thumbnail(pool, &post.post_id, &url).await {
                error!(error = ?e, post_id = post.post_id, "patreon: failed to cache thumbnail");
            }
            post.thumbnail_url = Some(url);
        }

        let rows = match PatreonAnnounceRow::for_post(
            pool,
            &post.campaign_id,
            post.is_public,
        )
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                error!(
                    error = ?e,
                    campaign_id = post.campaign_id,
                    "patreon: failed to load announce rows; the claimed item is dropped, not retried"
                );
                continue;
            },
        };

        if rows.is_empty() {
            warn!(
                post_id = post.post_id,
                campaign_id = post.campaign_id,
                is_public = post.is_public,
                "patreon: no guild announces this post; dropped"
            );
            continue;
        }

        broadcast(http, &post, rows).await;
    }

    Ok(())
}

async fn lookup_thumbnail(client: &Client, post: &PendingPost) -> Option<String> {
    if let Some(url) = thumbnail::fetch(client, &post.url).await {
        return Some(url);
    }

    if !post.is_public {
        return None;
    }

    let url = post.content_html.as_deref().and_then(thumbnail::content_image);
    if url.is_none() {
        info!(post_id = post.post_id, "patreon: no thumbnail found for post");
    }
    url
}

async fn broadcast(http: &Http, post: &PendingPost, rows: Vec<PatreonAnnounceRow>) {
    let component = embeds::post_component(post);

    futures::stream::iter(rows.into_iter().map(|row| {
        let component = component.clone();
        async move {
            let message = CreateMessage::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component]);

            match row.channel().widen().send_message(http, message).await {
                Ok(sent) => info!(
                    guild_id = row.guild_id,
                    channel_id = row.channel_id,
                    post_id = post.post_id,
                    message_id = %sent.id,
                    "patreon: announcement posted"
                ),
                Err(e) => error!(
                    error = %e,
                    guild_id = row.guild_id,
                    channel_id = row.channel_id,
                    post_id = post.post_id,
                    "patreon: failed to post announcement; it is not retried. \
                     Missing Permissions means the bot needs View Channel and \
                     Send Messages in that channel"
                ),
            }
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .for_each(|()| async {})
    .await;
}
