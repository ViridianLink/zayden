use std::time::Duration;

use reqwest::Client;
use scraper::{Html, Selector};
use tracing::debug;

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

pub async fn fetch(client: &Client, post_url: &str) -> Option<String> {
    let response = client
        .get(post_url)
        .timeout(TIMEOUT)
        .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);

    let html = match response {
        Ok(response) => response.text().await.ok()?,
        Err(e) => {
            debug!(error = ?e, post_url, "patreon: thumbnail lookup failed");
            return None;
        },
    };

    og_image(&html)
}
