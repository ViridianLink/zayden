use std::time::Duration;

use jiff::Timestamp;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;
use zayden_core::{RetryBudget, retry};

use crate::error::{PatreonError, Result};
use crate::model::PatreonPost;

pub const API_ROOT: &str = "https://www.patreon.com/api/oauth2/v2";
pub const POST_FIELDS: &str = "title,url,published_at,content,is_public";
pub const PAGE_SIZE: &str = "20";

const TIMEOUT: Duration = Duration::from_secs(10);
const RETRY: RetryBudget = RetryBudget::new(3, Duration::from_millis(500));

fn is_transient(error: &PatreonError) -> bool {
    let PatreonError::Reqwest(error) = error else { return false };

    error.is_timeout()
        || error.is_connect()
        || error.is_request()
        || error.status().is_some_and(|status| {
            status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
        })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostsPage {
    pub posts: Vec<PatreonPost>,
    pub next_cursor: Option<String>,
}

pub async fn fetch_posts(
    client: &Client,
    access_token: &str,
    campaign_id: &str,
    cursor: Option<&str>,
) -> Result<PostsPage> {
    let url = format!("{API_ROOT}/campaigns/{campaign_id}/posts");

    let body = fetch_json(|| {
        let mut request = client
            .get(&url)
            .bearer_auth(access_token)
            .query(&[("fields[post]", POST_FIELDS), ("page[count]", PAGE_SIZE)]);

        if let Some(cursor) = cursor {
            request = request.query(&[("page[cursor]", cursor)]);
        }

        request
    })
    .await?;

    Ok(parse_posts_page(&body, campaign_id))
}

async fn fetch_json<F>(build: F) -> Result<Value>
where
    F: Fn() -> RequestBuilder + Send + Sync,
{
    retry(RETRY, is_transient, || {
        let request = build().timeout(TIMEOUT);

        async move {
            let response = request.send().await?;

            if matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ) {
                return Err(PatreonError::Unauthorized);
            }

            let body = response.error_for_status()?.json::<Value>().await?;

            Ok(body)
        }
    })
    .await
}

#[must_use]
pub fn parse_posts_page(body: &Value, campaign_id: &str) -> PostsPage {
    let posts = body
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|resource| resource_to_post(resource, campaign_id))
        .collect();

    PostsPage { posts, next_cursor: next_cursor(body) }
}

#[must_use]
pub fn resource_to_post(
    resource: &Value,
    fallback_campaign: &str,
) -> Option<PatreonPost> {
    let id = resource.get("id").and_then(Value::as_str)?.to_owned();
    let attributes = resource.get("attributes")?;

    let url = attributes.get("url").and_then(Value::as_str)?.to_owned();
    let published_at = attributes
        .get("published_at")
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<Timestamp>().ok())?;

    let campaign_id = related_id(resource, "campaign")
        .unwrap_or_else(|| fallback_campaign.to_owned());

    Some(PatreonPost {
        id,
        campaign_id,
        title: attributes.get("title").and_then(Value::as_str).map(str::to_owned),
        url,
        content_html: attributes
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_owned),
        is_public: attributes
            .get("is_public")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        published_at,
    })
}

fn related_id(resource: &Value, name: &str) -> Option<String> {
    resource
        .get("relationships")?
        .get(name)?
        .get("data")?
        .get("id")?
        .as_str()
        .map(str::to_owned)
}

fn next_cursor(body: &Value) -> Option<String> {
    body.get("meta")?
        .get("pagination")?
        .get("cursors")?
        .get("next")?
        .as_str()
        .filter(|cursor| !cursor.is_empty())
        .map(str::to_owned)
}

pub async fn fetch_campaign(
    client: &Client,
    access_token: &str,
) -> Result<(String, Option<String>)> {
    let body = fetch_json(|| {
        client
            .get(format!("{API_ROOT}/campaigns"))
            .bearer_auth(access_token)
            .query(&[("fields[campaign]", "creation_name")])
    })
    .await?;

    let campaign = body
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
        .ok_or(PatreonError::NoCampaign)?;

    let id = campaign
        .get("id")
        .and_then(Value::as_str)
        .ok_or(PatreonError::NoCampaign)?
        .to_owned();

    let name = campaign
        .get("attributes")
        .and_then(|attributes| attributes.get("creation_name"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok((id, name))
}
