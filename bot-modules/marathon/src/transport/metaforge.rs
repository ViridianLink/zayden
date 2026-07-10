use reqwest::Client;
use serde_json::Value;

use crate::error::Result;

const REST: &str = "https://sb.metaforge.app/rest/v1";
const ANON_KEY: &str = "sb_publishable_C7SqVOoZBPFy4W0DxKcOGQ_emEIw-rj";
const MARKER_TABLE: &str = "marathon_map_data";

pub struct MetaForge {
    client: Client,
}

impl MetaForge {
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn map_markers(&self, slug: &str) -> Result<Vec<Value>> {
        let rows: Vec<Value> = self
            .client
            .get(format!("{REST}/{MARKER_TABLE}"))
            .query(&[
                ("map", format!("eq.{slug}")),
                ("select", "category,subcategory,instance_name".to_string()),
            ])
            .header("apikey", ANON_KEY)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(rows)
    }
}
