mod clearance;
mod listing;
mod preloaded_state;

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;

use self::clearance::Clearance;
use crate::error::{MarathonError, Result};
use crate::gql;

pub struct Mobalytics {
    client: Client,
    flaresolverr_url: String,
    clearance: Mutex<Option<Clearance>>,
}

impl Mobalytics {
    #[must_use]
    pub fn new(client: Client, flaresolverr_url: String) -> Self {
        Self { client, flaresolverr_url, clearance: Mutex::new(None) }
    }

    pub async fn fetch_document(&self, slug: &str) -> Result<Value> {
        let html = self
            .get_with_clearance(&format!("https://mobalytics.gg/marathon/{slug}"))
            .await?;
        let marathon_state = preloaded_state::extract_marathon_state(&html)?;
        gql::find_struct_document(&marathon_state, slug).cloned().ok_or_else(|| {
            MarathonError::NotFound {
                entity: "marathon document",
                query: slug.to_string(),
            }
        })
    }

    pub async fn fetch_ug_document(
        &self,
        category: &str,
        slug: &str,
    ) -> Result<Value> {
        let html = self
            .get_with_clearance(&format!(
                "https://mobalytics.gg/marathon/{category}/{slug}"
            ))
            .await?;
        let marathon_state = preloaded_state::extract_marathon_state(&html)?;
        gql::find_ug_document(&marathon_state, category, slug).cloned().ok_or_else(
            || MarathonError::NotFound {
                entity: "marathon document",
                query: slug.to_string(),
            },
        )
    }

    pub async fn fetch_listing_slugs(
        &self,
        path: &str,
        prefix: &str,
    ) -> Result<Vec<String>> {
        let clearance =
            clearance::ensure(&self.client, &self.flaresolverr_url, &self.clearance)
                .await?;
        let html = self
            .get_raw(&format!("https://mobalytics.gg/marathon/{path}"), &clearance)
            .await?;
        Ok(listing::extract_listing_slugs(&html, prefix))
    }

    async fn get_with_clearance(&self, url: &str) -> Result<String> {
        let clearance =
            clearance::ensure(&self.client, &self.flaresolverr_url, &self.clearance)
                .await?;
        let html = self.get_raw(url, &clearance).await?;
        if preloaded_state::contains_preloaded_state(&html) {
            return Ok(html);
        }

        let fresh =
            clearance::mint(&self.client, &self.flaresolverr_url, &self.clearance)
                .await?;
        let html = self.get_raw(url, &fresh).await?;
        if !preloaded_state::contains_preloaded_state(&html) {
            return Err(MarathonError::CloudflareChallenge);
        }
        Ok(html)
    }

    async fn get_raw(&self, url: &str, clearance: &Clearance) -> Result<String> {
        let resp = self
            .client
            .get(url)
            .header(
                reqwest::header::COOKIE,
                format!("cf_clearance={}", clearance.cookie),
            )
            .header(reqwest::header::USER_AGENT, &clearance.user_agent)
            .send()
            .await?;
        Ok(resp.text().await?)
    }
}
