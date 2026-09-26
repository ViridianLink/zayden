use reqwest::Client;
use sqlx::PgPool;
use tracing::warn;

use crate::oauth::{self, PatreonApp};
use crate::store::{PatreonCampaignRow, PatreonConnection};
use crate::webhook;

pub async fn release(
    client: &Client,
    pool: &PgPool,
    app: Option<&PatreonApp>,
    connection: &PatreonConnection,
) {
    if let Some(app) = app
        && let Some(webhook_id) = connection.webhook_id.as_deref()
    {
        match oauth::access_token(pool, client, app, connection).await {
            Ok(token) => webhook::unregister(client, &token, webhook_id).await,
            Err(e) => warn!(
                ?e,
                guild_id = connection.guild_id,
                "patreon: no usable token to remove the webhook"
            ),
        }
    }

    forget_campaign(pool, &connection.campaign_id).await;
}

pub async fn forget_campaign(pool: &PgPool, campaign_id: &str) {
    if let Err(e) = PatreonCampaignRow::forget(pool, campaign_id).await {
        warn!(?e, campaign_id, "patreon: failed to delete a released campaign");
    }
}
