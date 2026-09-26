use std::time::Duration;

use jiff::Timestamp;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;
use tracing::warn;
use url::Url;
use zayden_core::{RetryBudget, retry};

use crate::error::{PatreonError, Result};
use crate::model::PatreonPost;

pub const API_ROOT: &str = "https://www.patreon.com/api/oauth2/v2";
pub const WEB_ROOT: &str = "https://www.patreon.com";
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
                warn!(
                    status = %response.status(),
                    url = %response.url(),
                    "patreon: API rejected the access token"
                );
                return Err(PatreonError::Unauthorized);
            }

            if let Some(error) = response.error_for_status_ref().err() {
                let url = response.url().clone();
                let body = response.text().await.unwrap_or_default();
                warn!(
                    status = ?error.status(),
                    %url,
                    body = truncate_log(&body),
                    "patreon: API request failed"
                );
                return Err(error.into());
            }

            let body = response.json::<Value>().await?;

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
        .filter_map(|resource| {
            let post = resource_to_post(resource, campaign_id);
            if post.is_none() {
                let post_id = resource.get("id").and_then(Value::as_str);
                warn!(
                    campaign_id,
                    ?post_id,
                    "patreon: skipping a post missing its id, url or published_at"
                );
            }
            post
        })
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

    let url =
        attributes.get("url").and_then(Value::as_str).and_then(absolute_url)?;
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

fn absolute_url(raw: &str) -> Option<String> {
    Url::parse(WEB_ROOT).ok()?.join(raw).ok().map(String::from)
}

#[must_use]
pub fn truncate_log(body: &str) -> &str {
    const LIMIT: usize = 500;

    body.char_indices()
        .nth(LIMIT)
        .and_then(|(end, _)| body.get(..end))
        .unwrap_or(body)
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
        client.get(format!("{API_ROOT}/campaigns")).bearer_auth(access_token).query(
            &[
                ("include", "creator"),
                ("fields[campaign]", "vanity"),
                ("fields[user]", "full_name,vanity"),
            ],
        )
    })
    .await?;

    parse_campaign(&body)
}

pub fn parse_campaign(body: &Value) -> Result<(String, Option<String>)> {
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

    let creator_id = related_id(campaign, "creator");
    let creator = body
        .get("included")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|resource| {
            resource.get("type").and_then(Value::as_str) == Some("user")
                && (creator_id.is_none()
                    || resource.get("id").and_then(Value::as_str)
                        == creator_id.as_deref())
        })
        .and_then(|user| user.get("attributes"));

    let name = [
        creator.and_then(|a| a.get("full_name")),
        creator.and_then(|a| a.get("vanity")),
        campaign.get("attributes").and_then(|a| a.get("vanity")),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .map(str::trim)
    .find(|name| !name.is_empty())
    .map(str::to_owned);

    if name.is_none() {
        warn!(campaign_id = id, "patreon: campaign has no readable creator name");
    }

    Ok((id, name))
}
