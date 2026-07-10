use reqwest::Client;

use super::cloudflare::fetch_html;
use crate::parse;

pub struct PalDb {
    client: Client,
    flaresolverr_url: Option<String>,
}

impl PalDb {
    #[must_use]
    pub const fn new(client: Client, flaresolverr_url: Option<String>) -> Self {
        Self { client, flaresolverr_url }
    }

    pub async fn pal_description(&self, name: &str) -> Option<String> {
        let slug = name.replace(' ', "_");
        let url = format!("https://paldb.cc/en/{slug}");
        let html = fetch_html(&self.client, self.flaresolverr_url.as_deref(), &url)
            .await
            .ok()?;
        parse::og_description(&html)
    }
}
