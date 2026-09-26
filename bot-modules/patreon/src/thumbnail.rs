use std::time::Duration;

use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, warn};

const TIMEOUT: Duration = Duration::from_secs(8);

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
     AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

#[must_use]
pub fn og_image(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(r#"meta[property="og:image"]"#).ok()?;

    let content = document
        .select(&selector)
        .next()?
        .value()
        .attr("content")?
        .trim()
        .to_owned();

    (!content.is_empty()).then_some(content)
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

pub async fn fetch(client: &Client, post_url: &str) -> Option<String> {
    let response = client
        .get(post_url)
        .timeout(TIMEOUT)
        .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);

    let html = match response {
        Ok(response) => match response.text().await {
            Ok(html) => html,
            Err(e) => {
                warn!(error = ?e, post_url, "patreon: thumbnail page body unreadable");
                return None;
            },
        },
        Err(e) => {
            warn!(
                status = ?e.status(),
                error = %e,
                post_url,
                "patreon: thumbnail page fetch failed"
            );
            return None;
        },
    };

    let image = og_image(&html);
    if image.is_none() {
        debug!(post_url, "patreon: post page has no og:image");
    }
    image
}
