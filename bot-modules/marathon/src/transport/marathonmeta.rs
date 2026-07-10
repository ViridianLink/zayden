use reqwest::Client;

use super::cloudflare;
use crate::error::Result;

const BASE: &str = "https://marathonmeta.gg";

pub struct MarathonMeta {
    client: Client,
    flaresolverr_url: String,
}

impl MarathonMeta {
    #[must_use]
    pub const fn new(client: Client, flaresolverr_url: String) -> Self {
        Self { client, flaresolverr_url }
    }

    pub async fn weapon(&self, slug: &str) -> Result<String> {
        self.rendered(&format!("{BASE}/weapons/{slug}")).await
    }

    pub async fn runner(&self, slug: &str) -> Result<String> {
        self.rendered(&format!("{BASE}/runners/{slug}")).await
    }

    async fn rendered(&self, url: &str) -> Result<String> {
        cloudflare::get_rendered(&self.client, &self.flaresolverr_url, url).await
    }
}
