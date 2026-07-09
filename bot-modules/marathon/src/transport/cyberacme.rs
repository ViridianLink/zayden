use reqwest::Client;
use serde_json::Value;

use crate::error::{MarathonError, Result};

const API: &str = "https://cyberacme.org/api";
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                                   (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

const PAGE_SIZE: u32 = 500;

pub struct CyberAcme {
    client: Client,
}

impl CyberAcme {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn item(&self, slug: &str) -> Result<Value> {
        self.fetch(&format!("{API}/items/{slug}"), "item").await
    }

    pub async fn items(&self) -> Result<Vec<Value>> {
        let data = self
            .fetch(&format!("{API}/items?pageSize={PAGE_SIZE}"), "items")
            .await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn factions(&self) -> Result<Vec<Value>> {
        let data = self.fetch(&format!("{API}/factions"), "factions").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn faction(&self, slug: &str) -> Result<Value> {
        self.fetch_envelope(&format!("{API}/factions/{slug}")).await
    }

    pub async fn runners(&self) -> Result<Vec<Value>> {
        let data = self.fetch(&format!("{API}/runners"), "runners").await?;
        Ok(data.as_array().cloned().unwrap_or_default())
    }

    pub async fn runner(&self, slug: &str) -> Result<Value> {
        self.runners()
            .await?
            .into_iter()
            .find(|r| r.get("slug").and_then(Value::as_str) == Some(slug))
            .ok_or_else(|| MarathonError::NotFound {
                entity: "cyberacme runner",
                query: slug.to_string(),
            })
    }

    async fn fetch(&self, url: &str, key: &str) -> Result<Value> {
        let mut envelope = self.fetch_envelope(url).await?;
        envelope.get_mut(key).map(Value::take).ok_or_else(|| {
            MarathonError::NotFound {
                entity: "cyberacme resource",
                query: url.into(),
            }
        })
    }

    async fn fetch_envelope(&self, url: &str) -> Result<Value> {
        let envelope: Value = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, BROWSER_USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .json()
            .await?;

        let ok = matches!(
            envelope.get("status").and_then(Value::as_str),
            Some("ok" | "success")
        );
        if !ok {
            return Err(MarathonError::NotFound {
                entity: "cyberacme resource",
                query: url.to_string(),
            });
        }
        Ok(envelope)
    }
}
