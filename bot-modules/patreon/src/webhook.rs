use hmac::{Hmac, KeyInit, Mac};
use md5::Md5;
use reqwest::Client;
use serde_json::{Value, json};

use crate::api::{self, API_ROOT};
use crate::error::{PatreonError, Result};
use crate::model::PatreonPost;

pub const PATREON_SIGNATURE_HEADER: &str = "x-patreon-signature";
pub const PATREON_EVENT_HEADER: &str = "x-patreon-event";

pub const POST_PUBLISH: &str = "posts:publish";

pub async fn register(
    client: &Client,
    access_token: &str,
    campaign_id: &str,
    uri: &str,
) -> Result<(String, String)> {
    let body = json!({
        "data": {
            "type": "webhook",
            "attributes": { "triggers": [POST_PUBLISH], "uri": uri },
            "relationships": {
                "campaign": { "data": { "type": "campaign", "id": campaign_id } }
            }
        }
    });

    let response = client
        .post(format!("{API_ROOT}/webhooks"))
        .bearer_auth(access_token)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let data = response
        .get("data")
        .ok_or_else(|| PatreonError::Payload("webhook has no `data`".to_owned()))?;

    let id = data
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| PatreonError::Payload("webhook has no id".to_owned()))?;

    let secret = data
        .get("attributes")
        .and_then(|attributes| attributes.get("secret"))
        .and_then(Value::as_str)
        .ok_or_else(|| PatreonError::Payload("webhook has no secret".to_owned()))?;

    Ok((id.to_owned(), secret.to_owned()))
}

pub async fn unregister(client: &Client, access_token: &str, webhook_id: &str) {
    let result = client
        .delete(format!("{API_ROOT}/webhooks/{webhook_id}"))
        .bearer_auth(access_token)
        .send()
        .await;

    if let Err(e) = result {
        tracing::warn!(error = ?e, webhook_id, "patreon: failed to delete webhook");
    }
}

#[must_use]
pub fn verify_any(body: &[u8], signature: &str, secrets: &[String]) -> bool {
    secrets.iter().any(|secret| verify(body, signature, secret))
}

#[must_use]
pub fn verify(body: &[u8], signature: &str, secret: &str) -> bool {
    let Some(expected) = decode_hex(signature) else { return false };

    let Ok(mut mac) = Hmac::<Md5>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    mac.verify_slice(&expected).is_ok()
}

fn decode_hex(input: &str) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        return None;
    }

    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let hex = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(hex, 16).ok()
        })
        .collect()
}

pub fn parse_post(body: &[u8]) -> Result<PatreonPost> {
    let payload = serde_json::from_slice::<Value>(body)
        .map_err(|e| PatreonError::Payload(e.to_string()))?;

    let resource = payload
        .get("data")
        .ok_or_else(|| PatreonError::Payload("payload has no `data`".to_owned()))?;

    api::resource_to_post(resource, "")
        .filter(|post| !post.campaign_id.is_empty())
        .ok_or_else(|| {
            PatreonError::Payload(
                "post is missing an id, url, campaign or timestamp".to_owned(),
            )
        })
}
