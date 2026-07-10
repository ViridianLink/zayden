use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::error::Result;

const BASE: &str = "https://marathon-guide.com";
const PAGE_TTL: Duration = Duration::from_hours(8);
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub struct MarathonGuide {
    client: Client,
    weapons_page: Cache<(), Arc<str>>,
}

impl MarathonGuide {
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            weapons_page: Cache::builder()
                .time_to_live(PAGE_TTL)
                .max_capacity(1)
                .build(),
        }
    }

    pub async fn weapons(&self) -> Result<Arc<str>> {
        if let Some(cached) = self.weapons_page.get(&()).await {
            return Ok(cached);
        }
        let html = self.fetch(&format!("{BASE}/weapons/card")).await?;
        let page: Arc<str> = Arc::from(html);
        self.weapons_page.insert((), Arc::clone(&page)).await;
        Ok(page)
    }

    pub async fn runner(&self, slug: &str) -> Result<String> {
        self.fetch(&format!("{BASE}/runners/{slug}")).await
    }

    async fn fetch(&self, url: &str) -> Result<String> {
        let text = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }
}
