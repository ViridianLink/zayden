use std::time::Duration;

use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use tracing::{debug, warn};

const TIMEOUT: Duration = Duration::from_secs(8);

const POST_API: &str = "https://www.patreon.com/api/posts";

#[derive(Deserialize)]
struct PostResponse {
    data: PostData,
}

#[derive(Deserialize)]
struct PostData {
    attributes: PostAttributes,
}

#[derive(Deserialize)]
struct PostAttributes {
    image: Option<PostImage>,
}

#[derive(Deserialize)]
struct PostImage {
    large_url: Option<String>,
    url: Option<String>,
}

#[must_use]
pub fn api_image(json: &str) -> Option<String> {
    let image =
        serde_json::from_str::<PostResponse>(json).ok()?.data.attributes.image?;

    [image.large_url, image.url]
        .into_iter()
        .flatten()
        .map(|url| url.trim().to_owned())
        .find(|url| url.starts_with("https://"))
}

#[must_use]
pub fn content_image(content_html: &str) -> Option<String> {
    let fragment = Html::parse_fragment(content_html);
    let selector = Selector::parse("img[src]").ok()?;

    fragment
        .select(&selector)
        .filter_map(|img| img.value().attr("src"))
        .map(str::trim)
        .find(|src| src.starts_with("https://"))
        .map(str::to_owned)
}

pub async fn fetch(client: &Client, post_id: &str) -> Option<String> {
    let response = client
        .get(format!("{POST_API}/{post_id}"))
        .timeout(TIMEOUT)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);

    let json = match response {
        Ok(response) => match response.text().await {
            Ok(json) => json,
            Err(e) => {
                warn!(error = ?e, post_id, "patreon: thumbnail lookup body unreadable");
                return None;
            },
        },
        Err(e) => {
            warn!(
                status = ?e.status(),
                error = %e,
                post_id,
                "patreon: thumbnail lookup failed"
            );
            return None;
        },
    };

    let image = api_image(&json);
    if image.is_none() {
        debug!(post_id, "patreon: post has no image");
    }
    image
}
