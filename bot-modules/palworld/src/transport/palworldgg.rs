use reqwest::Client;

use super::cloudflare::fetch_html;
use crate::parse;

pub struct PalworldGg {
    client: Client,
    flaresolverr_url: Option<String>,
}

impl PalworldGg {
    #[must_use]
    pub const fn new(client: Client, flaresolverr_url: Option<String>) -> Self {
        Self { client, flaresolverr_url }
    }

    pub async fn pal_description(&self, slug: &str) -> Option<String> {
        let url = format!("https://palworld.gg/pal/{slug}");
        let html = fetch_html(&self.client, self.flaresolverr_url.as_deref(), &url)
            .await
            .ok()?;
        parse::og_description(&html)
    }
}
