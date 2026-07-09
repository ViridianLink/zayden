use std::sync::Arc;

use serde_json::Value;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::merge::Merge;
use crate::model::MarathonMap;
use crate::parse;

impl MarathonClient {
    pub async fn map(&self, slug: &str) -> Result<Arc<MarathonMap>> {
        if let Some(cached) = self.map_cache.get(slug).await {
            return Ok(cached);
        }
        let map = self.cross_reference_map(slug).await?;
        let entry = Arc::new(map);
        self.map_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn cross_reference_map(&self, slug: &str) -> Result<MarathonMap> {
        let (marathondb, mapgenie) =
            tokio::join!(self.marathondb_map(slug), self.mapgenie_map(slug));

        let mut merged: Option<MarathonMap> = None;
        fold_source(&mut merged, marathondb, slug);
        fold_source(&mut merged, mapgenie, slug);

        merged.ok_or_else(|| MarathonError::NotFound {
            entity: "map",
            query: slug.to_string(),
        })
    }

    async fn marathondb_map(&self, slug: &str) -> Result<MarathonMap> {
        let data = self.marathondb.map(slug).await?;
        Ok(parse::marathondb_map_to_model(slug, &data))
    }

    async fn mapgenie_map(&self, slug: &str) -> Result<MarathonMap> {
        let doc = self.mapgenie.map(slug).await?;
        Ok(parse::mapgenie_map_to_model(slug, &doc.taxonomy, &doc.data))
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
        let mut slugs: Vec<String> = Vec::new();
        let mut push = |slug: String| {
            if !slugs.contains(&slug) {
                slugs.push(slug);
            }
        };

        match self.marathondb.maps().await {
            Ok(items) => items
                .iter()
                .filter_map(|m| {
                    m.get("slug").and_then(Value::as_str).map(str::to_string)
                })
                .for_each(&mut push),
            Err(err) => tracing::debug!(%err, "marathondb map list unavailable"),
        }

        match self.mapgenie.slugs().await {
            Ok(mg) => mg.into_iter().for_each(&mut push),
            Err(err) => tracing::debug!(%err, "mapgenie roster unavailable"),
        }

        if slugs.is_empty() {
            return Err(MarathonError::SourceUnavailable);
        }
        Ok(slugs)
    }
}

fn fold_source(
    merged: &mut Option<MarathonMap>,
    candidate: Result<MarathonMap>,
    slug: &str,
) {
    match candidate {
        Ok(map) => match merged {
            Some(existing) => existing.merge_from(map),
            None => *merged = Some(map),
        },
        Err(err) => tracing::debug!(%slug, %err, "map source unavailable"),
    }
}
