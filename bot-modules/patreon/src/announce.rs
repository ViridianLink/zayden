use futures::StreamExt;
use jiff::{SignedDuration, Timestamp};
use reqwest::Client;
use serenity::all::{CreateMessage, Http, MessageFlags};
use sqlx::PgPool;
use tracing::{debug, error};

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

    let cutoff = Timestamp::now() - MAX_AGE;

    for mut post in pending {
        if post.published_at.to_jiff() < cutoff {
            debug!(post_id = post.post_id, "patreon: skipping a stale post");
            continue;
        }

        if post.thumbnail_url.is_none()
            && let Some(url) = thumbnail::fetch(client, &post.url).await
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

        broadcast(http, &post, rows).await;
    }

    Ok(())
}

async fn broadcast(http: &Http, post: &PendingPost, rows: Vec<PatreonAnnounceRow>) {
    let component = embeds::post_component(post);

    futures::stream::iter(rows.into_iter().map(|row| {
        let component = component.clone();
        async move {
            let message = CreateMessage::new()
                .flags(MessageFlags::IS_COMPONENTS_V2)
                .components(vec![component]);

            if let Err(e) = row.channel().widen().send_message(http, message).await {
                error!(
                    error = ?e,
                    guild_id = row.guild_id,
                    post_id = post.post_id,
                    "patreon: failed to post announcement; it is not retried"
                );
            }
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .for_each(|()| async {})
    .await;
}
