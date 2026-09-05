use std::sync::Arc;

use reqwest::Client;
use sqlx::PgPool;
use tracing::{debug, error, warn};
use zayden_core::CronJob;

use crate::error::{PatreonError, Result};
use crate::oauth::{self, PatreonApp};
use crate::store::{self, PatreonCampaignRow, PatreonConnection};
use crate::{announce, api};

const MAX_PAGES: usize = 25;
const FAILURE_ALERT: i32 = 5;

pub struct PatreonPollCron;

impl PatreonPollCron {
    pub fn cron_job(
        client: Client,
        app: Arc<PatreonApp>,
    ) -> std::result::Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("patreon_poll", "0 */15 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = client.clone();
                let app = Arc::clone(&app);

                async move {
                    poll_all(&client, &app, &pool).await;

                    if let Err(e) =
                        announce::announce_pending(&ctx.http, &client, &pool).await
                    {
                        error!(error = ?e, "patreon: failed to announce pending posts");
                    }
                }
            })
        })
    }
}

async fn poll_all(client: &Client, app: &PatreonApp, pool: &PgPool) {
    let connections = match PatreonConnection::pollable(pool).await {
        Ok(connections) => connections,
        Err(e) => {
            error!(error = ?e, "patreon: failed to load connections");
            return;
        },
    };

    if connections.is_empty() {
        debug!("patreon: no guild has a connected campaign, skipping poll");
        return;
    }

    for connection in connections {
        // Each guild holds its own grant, so one dead connection must not stop
        // the others. `access_token` has already logged and disabled it.
        let access_token =
            match oauth::access_token(pool, client, app, &connection).await {
                Ok(token) => token,
                Err(e) => {
                    warn!(
                        error = ?e,
                        guild_id = connection.guild_id,
                        "patreon: no usable access token, skipping campaign"
                    );
                    continue;
                },
            };

        let campaign = match PatreonCampaignRow::select(
            pool,
            &connection.campaign_id,
        )
        .await
        {
            Ok(Some(campaign)) => campaign,
            Ok(None) => {
                warn!(
                    campaign_id = connection.campaign_id,
                    "patreon: connection references a campaign with no poll state"
                );
                continue;
            },
            Err(e) => {
                error!(error = ?e, "patreon: failed to load campaign state");
                continue;
            },
        };

        match poll_campaign(client, pool, &access_token, &campaign).await {
            Ok(stored) => {
                debug!(
                    campaign_id = campaign.campaign_id,
                    stored, "patreon: campaign polled"
                );
            },
            Err(e) => on_failure(pool, &campaign.campaign_id, &e).await,
        }
    }
}

async fn poll_campaign(
    client: &Client,
    pool: &PgPool,
    access_token: &str,
    campaign: &PatreonCampaignRow,
) -> Result<usize> {
    let seeded = campaign.is_seeded();
    let mut cursor = campaign.next_cursor.clone();
    let mut resume_from = cursor.clone();
    let mut stored = 0_usize;

    for _page in 0..MAX_PAGES {
        let page = api::fetch_posts(
            client,
            access_token,
            &campaign.campaign_id,
            cursor.as_deref(),
        )
        .await?;

        for post in &page.posts {
            if store::insert_post(pool, post, !seeded).await? {
                stored += 1;
            }
        }

        let Some(next) = page.next_cursor else { break };

        resume_from = Some(next.clone());
        cursor = Some(next);
    }

    PatreonCampaignRow::record_success(
        pool,
        &campaign.campaign_id,
        resume_from.as_deref(),
    )
    .await?;

    Ok(stored)
}

async fn on_failure(pool: &PgPool, campaign_id: &str, error: &PatreonError) {
    let failures = match PatreonCampaignRow::record_failure(pool, campaign_id).await
    {
        Ok(failures) => failures,
        Err(e) => {
            error!(error = ?e, campaign_id, "patreon: failed to record poll failure");
            return;
        },
    };

    if failures >= FAILURE_ALERT {
        error!(
            error = ?error,
            campaign_id,
            failures,
            "patreon: campaign has failed to poll repeatedly"
        );
    } else {
        warn!(error = ?error, campaign_id, failures, "patreon: campaign poll failed");
    }
}
