use std::sync::Arc;

use reqwest::Client;
use sqlx::PgPool;
use tracing::{debug, error, warn};
use zayden_core::CronJob;

use crate::store::YoutubeChannelRow;
use crate::websub::Mode;
use crate::{announce, poll, websub};

pub struct YoutubePollCron;

impl YoutubePollCron {
    pub fn cron_job(
        client: Client,
        api_key: Arc<str>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("youtube_poll", "0 */15 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = client.clone();
                let api_key = Arc::clone(&api_key);

                async move {
                    poll_all(&client, &pool, &api_key).await;

                    if let Err(e) = announce::announce_pending(&ctx.http, &pool).await {
                        error!(error = ?e, "youtube: failed to announce pending videos");
                    }
                }
            })
        })
    }
}

async fn poll_all(client: &Client, pool: &PgPool, api_key: &str) {
    let channels = match YoutubeChannelRow::pollable(pool).await {
        Ok(channels) => channels,
        Err(e) => {
            error!(error = ?e, "youtube: failed to load channels");
            return;
        },
    };

    for channel in channels {
        match poll::poll_channel(client, pool, api_key, &channel).await {
            Ok(stored) => {
                debug!(
                    channel_id = channel.channel_id,
                    stored, "youtube: channel polled"
                );
            },
            Err(e) => poll::on_failure(pool, &channel.channel_id, &e).await,
        }
    }
}

pub struct YoutubeLeaseCron;

impl YoutubeLeaseCron {
    pub fn cron_job(
        client: Client,
        webhook_uri: Arc<str>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("youtube_websub_lease", "0 0 */6 * * * *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let client = client.clone();
                let webhook_uri = Arc::clone(&webhook_uri);

                async move { renew_all(&client, &pool, &webhook_uri).await }
            })
        })
    }
}

async fn renew_all(client: &Client, pool: &PgPool, webhook_uri: &str) {
    let channels = match YoutubeChannelRow::lease_due(pool).await {
        Ok(channels) => channels,
        Err(e) => {
            error!(error = ?e, "youtube: failed to load WebSub leases");
            return;
        },
    };

    for channel in channels {
        if let Err(e) = websub::request(
            client,
            Mode::Subscribe,
            webhook_uri,
            &channel.channel_id,
            &channel.websub_secret,
        )
        .await
        {
            warn!(
                error = ?e,
                channel_id = channel.channel_id,
                "youtube: WebSub renewal failed; the poll still covers this channel"
            );
        }
    }
}
