use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{MarathonError, Result};

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

#[derive(Debug, Deserialize)]
struct MarathonDbEnvelope {
    success: bool,
    data: Value,
}

pub struct MarathonDb {
    client: Client,
}

impl MarathonDb {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn weapon(&self, slug: &str) -> Result<Value> {
        self.fetch(&format!("https://weapons.marathondb.gg/api/weapons/{slug}"))
            .await
    }

    pub async fn runner(&self, slug: &str) -> Result<Value> {
        self.fetch(&format!("https://runners.marathondb.gg/api/runners/{slug}"))
            .await
    }

    pub async fn contracts(&self) -> Result<Vec<Value>> {
        let data = self.fetch("https://helpbot.marathondb.gg/api/contracts").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    async fn fetch(&self, url: &str) -> Result<Value> {
        let envelope: MarathonDbEnvelope = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .json()
            .await?;

        if !envelope.success {
            return Err(MarathonError::NotFound {
                entity: "marathondb resource",
                query: url.to_string(),
            });
        }
        Ok(envelope.data)
    }
}
