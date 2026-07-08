use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::Cradle;
use crate::parse;

impl MarathonClient {
    pub async fn cradle(&self) -> Result<Arc<Cradle>> {
        if let Some(cached) = self.cradle_cache.get(&()).await {
            return Ok(cached);
        }
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };

        let doc = match mobalytics.fetch_document("cradle").await {
            Ok(doc) => doc,
            Err(_) => mobalytics.fetch_document("builds").await?,
        };
        let cradle = parse::parse_cradle(&doc);
        let entry = Arc::new(cradle);
        self.cradle_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }
}
