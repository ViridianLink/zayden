use reqwest::Client;
use serde_json::Value;

use crate::error::Result;

const API: &str = "https://palworld.fandom.com/api.php";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub struct Fandom {
    client: Client,
}

impl Fandom {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn description(&self, title: &str) -> Option<String> {
        self.try_description(title).await.ok().flatten()
    }

    async fn try_description(&self, title: &str) -> Result<Option<String>> {
        let body: Value = self
            .client
            .get(API)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .query(&[
                ("action", "query"),
                ("prop", "extracts"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("redirects", "1"),
                ("format", "json"),
                ("titles", title),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(extract_first_page_extract(&body))
    }
}

fn extract_first_page_extract(body: &Value) -> Option<String> {
    let pages = body.get("query")?.get("pages")?.as_object()?;
    pages.values().find_map(|page| {
        let extract = page.get("extract").and_then(Value::as_str)?.trim();
        (!extract.is_empty()).then(|| extract.to_string())
    })
}
