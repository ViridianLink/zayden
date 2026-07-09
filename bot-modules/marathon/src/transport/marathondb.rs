use reqwest::Client;
use serde_json::Value;

use crate::error::{MarathonError, Result};

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

pub struct MarathonDb {
    client: Client,
}

impl MarathonDb {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn weapon(&self, slug: &str) -> Result<Value> {
        self.fetch(
            &format!("https://weapons.marathondb.gg/api/weapons/{slug}"),
            "data",
        )
        .await
    }

    pub async fn weapons(&self) -> Result<Vec<Value>> {
        let data =
            self.fetch("https://weapons.marathondb.gg/api/weapons", "data").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn runner(&self, slug: &str) -> Result<Value> {
        self.fetch(
            &format!("https://runners.marathondb.gg/api/runners/{slug}"),
            "data",
        )
        .await
    }

    pub async fn runners(&self) -> Result<Vec<Value>> {
        let data =
            self.fetch("https://runners.marathondb.gg/api/runners", "data").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn map(&self, slug: &str) -> Result<Value> {
        self.fetch(&format!("https://helpbot.marathondb.gg/api/maps/{slug}"), "map")
            .await
    }

    pub async fn maps(&self) -> Result<Vec<Value>> {
        let data =
            self.fetch("https://helpbot.marathondb.gg/api/maps", "maps").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn contracts(&self) -> Result<Vec<Value>> {
        let data = self
            .fetch("https://helpbot.marathondb.gg/api/contracts", "data")
            .await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    async fn fetch(&self, url: &str, key: &str) -> Result<Value> {
        let mut envelope: Value = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .json()
            .await?;

        let success = envelope.get("success").and_then(Value::as_bool) == Some(true);
        if !success {
            return Err(MarathonError::NotFound {
                entity: "marathondb resource",
                query: url.to_string(),
            });
        }

        envelope.get_mut(key).map(Value::take).ok_or_else(|| {
            MarathonError::NotFound {
                entity: "marathondb resource",
                query: url.to_string(),
            }
        })
    }
}
