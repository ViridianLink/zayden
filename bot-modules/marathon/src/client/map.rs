use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::MarathonMap;
use crate::parse;

impl MarathonClient {
    pub async fn map(&self, slug: &str) -> Result<Arc<MarathonMap>> {
        if let Some(cached) = self.map_cache.get(slug).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document(&format!("maps/{slug}")).await?;
        let map = parse::parse_map(slug, &doc);
        let entry = Arc::new(map);
        self.map_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    pub async fn maps(&self) -> Result<Arc<[MarathonMap]>> {
        if let Some(cached) = self.map_list_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let slugs = mobalytics.fetch_listing_slugs("maps", "maps").await?;

        let mut maps = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            maps.push((*self.map(slug).await?).clone());
        }

        let entry: Arc<[MarathonMap]> = maps.into();
        self.map_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }
}
