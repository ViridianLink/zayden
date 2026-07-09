use std::sync::Arc;

use serde_json::Value;

use super::MarathonClient;
use crate::error::Result;
use crate::model::MarathonMap;
use crate::parse;

impl MarathonClient {
    pub async fn map(&self, slug: &str) -> Result<Arc<MarathonMap>> {
        if let Some(cached) = self.map_cache.get(slug).await {
            return Ok(cached);
        }
        let map = self.fetch_map(slug).await?;
        let entry = Arc::new(map);
        self.map_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn fetch_map(&self, slug: &str) -> Result<MarathonMap> {
        match self.marathondb.map(slug).await {
            Ok(data) => Ok(parse::marathondb_map_to_model(slug, &data)),
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    let doc =
                        mobalytics.fetch_document(&format!("maps/{slug}")).await?;
                    Ok(parse::parse_map(slug, &doc))
                },
                None => Err(err),
            },
        }
    }

    pub async fn maps(&self) -> Result<Arc<[MarathonMap]>> {
        if let Some(cached) = self.map_list_cache.get(&()).await {
            return Ok(cached);
        }

        let slugs = self.map_slugs().await?;
        let mut maps = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            maps.push((*self.map(slug).await?).clone());
        }

        let entry: Arc<[MarathonMap]> = maps.into();
        self.map_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn map_slugs(&self) -> Result<Vec<String>> {
        match self.marathondb.maps().await {
            Ok(items) => Ok(items
                .iter()
                .filter_map(|m| {
                    m.get("slug").and_then(Value::as_str).map(str::to_string)
                })
                .collect()),
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    mobalytics.fetch_listing_slugs("maps", "maps").await
                },
                None => Err(err),
            },
        }
    }
}
