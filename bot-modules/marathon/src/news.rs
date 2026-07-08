use reqwest::Client;
use serde_json::Value;

use crate::error::Result;
use crate::model::NewsItem;

const BUNGIE_NEWS_URL: &str =
    "https://www.bungie.net/Platform/Content/Rss/NewsArticles/0/";
const BUNGIE_NEWS_SOURCE: &str = "bungie_news";

pub const BLUESKY_ACTORS: [&str; 3] = [
    "marathonthegame.bungie.net",
    "marathonteam.bungie.net",
    "bungieserverstatus.bungie.net",
];
const BLUESKY_ITEMS_PER_ACTOR: usize = 5;

pub struct BungieNewsFeed;

impl BungieNewsFeed {
    pub async fn fetch(client: &Client, api_key: &str) -> Result<Vec<NewsItem>> {
        let body: Value = client
            .get(BUNGIE_NEWS_URL)
            .header("X-API-Key", api_key)
            .send()
            .await?
            .json()
            .await?;

        Ok(find_articles(&body)
            .into_iter()
            .filter(|article| mentions_marathon(article))
            .filter_map(article_to_news_item)
            .collect())
    }
}

fn find_articles(body: &Value) -> Vec<&Value> {
    let response = body.get("Response").unwrap_or(body);

    for key in ["results", "NewsArticles", "articles", "Articles"] {
        if let Some(array) = response.get(key).and_then(Value::as_array) {
            return array.iter().collect();
        }
    }

    response.as_array().map_or_else(Vec::new, |array| array.iter().collect())
}

fn first_str<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| value.get(key).and_then(Value::as_str))
}

fn mentions_marathon(article: &Value) -> bool {
    const FIELDS: &[&str] =
        &["Title", "title", "Category", "category", "Description", "description"];

    FIELDS.iter().any(|key| {
        article
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| s.to_lowercase().contains("marathon"))
    })
}

fn article_to_news_item(article: &Value) -> Option<NewsItem> {
    let id = first_str(article, &["UniqueIdentifier", "Id", "id", "Link", "link"])?
        .to_string();
    let title = first_str(article, &["Title", "title"])?.to_string();
    let url =
        first_str(article, &["Link", "link", "OriginalLink"]).map(str::to_string);
    let summary =
        first_str(article, &["Description", "description", "Body", "body"])
            .map(str::to_string);

    Some(NewsItem {
        feed_key: BUNGIE_NEWS_SOURCE.to_string(),
        id,
        source_label: "Bungie News".to_string(),
        title,
        url,
        summary,
    })
}

pub struct BlueskyFeed;

impl BlueskyFeed {
    pub async fn fetch(client: &Client) -> Result<Vec<NewsItem>> {
        let mut items = Vec::new();
        for actor in BLUESKY_ACTORS {
            items.extend(Self::fetch_actor(client, actor).await?);
        }
        Ok(items)
    }

    pub async fn fetch_actor(client: &Client, actor: &str) -> Result<Vec<NewsItem>> {
        let body: Value = client
            .get("https://public.api.bsky.app/xrpc/app.bsky.feed.getAuthorFeed")
            .query(&[("actor", actor)])
            .send()
            .await?
            .json()
            .await?;

        Ok(body
            .get("feed")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|entry| !is_reply_or_repost(entry))
            .take(BLUESKY_ITEMS_PER_ACTOR)
            .filter_map(|entry| feed_entry_to_news_item(actor, entry))
            .collect())
    }
}

fn is_reply_or_repost(entry: &Value) -> bool {
    entry.get("reason").is_some() || entry.get("reply").is_some()
}

fn feed_entry_to_news_item(actor: &str, entry: &Value) -> Option<NewsItem> {
    let post = entry.get("post")?;
    let uri = post.get("uri").and_then(Value::as_str)?.to_string();
    let text = post
        .get("record")
        .and_then(|record| record.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let url = post_uri_to_url(actor, &uri);

    Some(NewsItem {
        feed_key: format!("bluesky:{actor}"),
        id: uri,
        source_label: format!("Bluesky (@{actor})"),
        title: text,
        url,
        summary: None,
    })
}

fn post_uri_to_url(actor: &str, uri: &str) -> Option<String> {
    let rkey = uri.rsplit('/').next()?;
    Some(format!("https://bsky.app/profile/{actor}/post/{rkey}"))
}

#[must_use]
pub fn new_since<'a>(
    items: &'a [NewsItem],
    last_id: Option<&str>,
) -> &'a [NewsItem] {
    let Some(last_id) = last_id else { return &[] };

    items
        .iter()
        .position(|item| item.id == last_id)
        .map_or(items, |idx| items.get(..idx).unwrap_or(&[]))
}
