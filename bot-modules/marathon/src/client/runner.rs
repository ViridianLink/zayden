use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::Runner;
use crate::parse;

impl MarathonClient {
    pub async fn runner(&self, slug: &str) -> Result<Arc<Runner>> {
        if let Some(cached) = self.runner_cache.get(slug).await {
            return Ok(cached);
        }
        let runner = self.fetch_runner(slug).await?;
        let entry = Arc::new(runner);
        self.runner_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn fetch_runner(&self, slug: &str) -> Result<Runner> {
        if let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) =
                mobalytics.fetch_document(&format!("runners/{slug}")).await
        {
            return Ok(parse::parse_runner(slug, &doc));
        }
        let data = self.marathondb.runner(slug).await?;
        Ok(parse::marathondb_runner_to_model(slug, &data))
    }

    pub async fn runners(&self) -> Result<Arc<[Runner]>> {
        if let Some(cached) = self.runner_list_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let slugs = mobalytics.fetch_listing_slugs("runners", "runners").await?;

        let mut runners = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            runners.push((*self.runner(slug).await?).clone());
        }

        let entry: Arc<[Runner]> = runners.into();
        self.runner_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }
}
