use reqwest::Client;

use super::cloudflare::fetch_html;
use crate::parse;

#[derive(Debug, Clone, Default)]
pub struct PalDetails {
    pub description: Option<String>,
    pub image_url: Option<String>,
}

pub struct PalDb {
    client: Client,
    flaresolverr_url: Option<String>,
}

impl PalDb {
    #[must_use]
    pub const fn new(client: Client, flaresolverr_url: Option<String>) -> Self {
        Self { client, flaresolverr_url }
    }

    pub async fn pal_details(&self, name: &str) -> PalDetails {
        let slug = name.replace(' ', "_");
        let url = format!("https://paldb.cc/en/{slug}");
        let Ok(html) =
            fetch_html(&self.client, self.flaresolverr_url.as_deref(), &url).await
        else {
            return PalDetails::default();
        };
        PalDetails {
            description: parse::og_description(&html),
            image_url: parse::og_image(&html),
        }
    }
}
