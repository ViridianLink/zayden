use reqwest::Client;
use serde_json::Value;

use super::cloudflare;
use crate::error::{MarathonError, Result};
use crate::parse::html;

const BASE: &str = "https://tauceti.gg";

pub struct TauCeti {
    client: Client,
    flaresolverr_url: String,
}

impl TauCeti {
    #[must_use]
    pub const fn new(client: Client, flaresolverr_url: String) -> Self {
        Self { client, flaresolverr_url }
    }

    pub async fn weapon(&self, slug: &str) -> Result<Value> {
        self.object(&format!("{BASE}/db/weapons/{slug}"), slug).await
    }

    pub async fn faction(&self, slug: &str) -> Result<Value> {
        self.object(&format!("{BASE}/factions/{slug}"), slug).await
    }

    pub async fn runner(&self, slug: &str) -> Result<Value> {
        self.object(&format!("{BASE}/runners/{slug}"), slug).await
    }

    async fn object(&self, url: &str, slug: &str) -> Result<Value> {
        let rendered =
            cloudflare::get_rendered(&self.client, &self.flaresolverr_url, url)
                .await?;

        let flight = html::next_flight(&html::document(&rendered));
        html::flight_object_by_slug(&flight, slug).ok_or_else(|| {
            MarathonError::NotFound {
                entity: "tauceti resource",
                query: slug.to_string(),
            }
        })
    }
}
