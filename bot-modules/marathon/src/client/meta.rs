use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::MetaEntry;
use crate::parse;

impl MarathonClient {
    pub async fn meta(&self) -> Result<Arc<[MetaEntry]>> {
        if let Some(cached) = self.meta_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };

        let mut entries = Vec::new();
        if let Ok(doc) = mobalytics.fetch_document("tier-lists").await {
            entries.extend(parse::parse_meta(&doc));
        }
        if let Ok(doc) = mobalytics.fetch_ug_document("tier-lists", "weapons").await
        {
            entries.extend(parse::parse_meta(&doc));
        }
        if entries.is_empty() {
            return Err(MarathonError::SourceUnavailable);
        }

        let entry: Arc<[MetaEntry]> = entries.into();
        self.meta_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }
}
