use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::BuildRecipe;
use crate::parse;

impl MarathonClient {
    pub async fn build(&self, slug: &str) -> Result<Arc<BuildRecipe>> {
        if let Some(cached) = self.build_cache.get(slug).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_ug_document("builds", slug).await?;
        let build = parse::parse_build(slug, &doc);
        let entry = Arc::new(build);
        self.build_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    pub async fn builds(&self) -> Result<Arc<[BuildRecipe]>> {
        if let Some(cached) = self.build_list_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let slugs = mobalytics.fetch_listing_slugs("builds", "builds").await?;

        let mut builds = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            builds.push((*self.build(slug).await?).clone());
        }

        let entry: Arc<[BuildRecipe]> = builds.into();
        self.build_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }
}
