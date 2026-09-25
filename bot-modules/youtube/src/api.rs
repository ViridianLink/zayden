use std::time::Duration;

use jiff::Timestamp;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;
use zayden_core::{RetryBudget, retry};

use crate::error::{Result, YoutubeError};
use crate::model::{OwnChannel, YoutubeVideo};

pub const API_ROOT: &str = "https://www.googleapis.com/youtube/v3";
pub const PAGE_SIZE: &str = "20";

const API_KEY_HEADER: &str = "x-goog-api-key";

const TIMEOUT: Duration = Duration::from_secs(10);
const RETRY: RetryBudget = RetryBudget::new(3, Duration::from_millis(500));

fn is_transient(error: &YoutubeError) -> bool {
    let YoutubeError::Reqwest(error) = error else { return false };

    error.is_timeout()
        || error.is_connect()
        || error.is_request()
        || error.status().is_some_and(|status| {
            status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
        })
}

pub async fn fetch_own_channel(
    client: &Client,
    access_token: &str,
) -> Result<OwnChannel> {
    let body = fetch_json(|| {
        client
            .get(format!("{API_ROOT}/channels"))
            .bearer_auth(access_token)
            .query(&[("part", "snippet,contentDetails"), ("mine", "true")])
    })
    .await?
    .ok_or(YoutubeError::NoChannel)?;

    parse_own_channel(&body)
}

pub fn parse_own_channel(body: &Value) -> Result<OwnChannel> {
    let channel = body
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or(YoutubeError::NoChannel)?;

    let id = channel
        .get("id")
        .and_then(Value::as_str)
        .ok_or(YoutubeError::NoChannel)?
        .to_owned();

    let title = channel
        .pointer("/snippet/title")
        .and_then(Value::as_str)
        .unwrap_or(&id)
        .to_owned();

    let uploads_playlist_id = channel
        .pointer("/contentDetails/relatedPlaylists/uploads")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            YoutubeError::Payload("channel has no uploads playlist".to_owned())
        })?
        .to_owned();

    Ok(OwnChannel { id, title, uploads_playlist_id })
}

pub async fn fetch_uploads(
    client: &Client,
    api_key: &str,
    playlist_id: &str,
    channel_id: &str,
) -> Result<Vec<YoutubeVideo>> {
    let body = fetch_json(|| {
        client
            .get(format!("{API_ROOT}/playlistItems"))
            .header(API_KEY_HEADER, api_key)
            .query(&[
                ("part", "snippet,contentDetails,status"),
                ("maxResults", PAGE_SIZE),
                ("playlistId", playlist_id),
            ])
    })
    .await?;

    // A channel that has never uploaded has no uploads playlist yet.
    Ok(body.map_or_default(|body| parse_uploads(&body, channel_id)))
}

async fn fetch_json<F>(build: F) -> Result<Option<Value>>
where
    F: Fn() -> RequestBuilder + Send + Sync,
{
    retry(RETRY, is_transient, || {
        let request = build().timeout(TIMEOUT);

        async move {
            let response = request.send().await?;

            match response.status() {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    return Err(YoutubeError::Unauthorized);
                },
                StatusCode::NOT_FOUND => return Ok(None),
                _ => {},
            }

            let body = response.error_for_status()?.json::<Value>().await?;

            Ok(Some(body))
        }
    })
    .await
}

#[must_use]
pub fn parse_uploads(body: &Value, channel_id: &str) -> Vec<YoutubeVideo> {
    body.get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item_to_video(item, channel_id))
        .collect()
}

fn item_to_video(item: &Value, channel_id: &str) -> Option<YoutubeVideo> {
    let privacy = item.pointer("/status/privacyStatus").and_then(Value::as_str);
    if privacy != Some("public") {
        return None;
    }

    let id = item
        .pointer("/contentDetails/videoId")
        .or_else(|| item.pointer("/snippet/resourceId/videoId"))
        .and_then(Value::as_str)?
        .to_owned();

    let title = item.pointer("/snippet/title").and_then(Value::as_str)?.to_owned();

    let published_at = item
        .pointer("/contentDetails/videoPublishedAt")
        .or_else(|| item.pointer("/snippet/publishedAt"))
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<Timestamp>().ok())?;

    Some(YoutubeVideo { id, channel_id: channel_id.to_owned(), title, published_at })
}
