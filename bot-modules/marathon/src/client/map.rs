use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, warn};

use super::MarathonClient;
use super::health::{SourceData, flag_degraded_sources};
use crate::error::{MarathonError, Result};
use crate::merge::Merge;
use crate::model::MarathonMap;
use crate::parse;
use crate::source::SourceId;

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
        let (marathondb, mapgenie, metaforge) = tokio::join!(
            self.marathondb_map(slug),
            self.mapgenie_map(slug),
            self.metaforge_map(slug),
        );

        let mut candidates: Vec<(SourceId, MarathonMap)> = Vec::new();
        collect_map(&mut candidates, SourceId::MarathonDb, marathondb, slug);
        collect_map(&mut candidates, SourceId::MapGenie, mapgenie, slug);
        collect_map(&mut candidates, SourceId::MetaForge, metaforge, slug);
        flag_degraded_sources(&candidates, "map", slug);

        let mut merged: Option<MarathonMap> = None;
        for (_, map) in candidates {
            match &mut merged {
                Some(existing) => existing.merge_from(map),
                None => merged = Some(map),
            }
        }

        merged.ok_or_else(|| MarathonError::NotFound {
            entity: "map",
            query: slug.to_string(),
        })
    }

    async fn marathondb_map(&self, slug: &str) -> Result<MarathonMap> {
        let data = self.marathondb.map(slug).await?;
        let map = parse::marathondb_map_to_model(slug, &data);
        if map.is_degraded() {
            return Err(MarathonError::NotFound {
                entity: "map",
                query: slug.to_string(),
            });
        }
        Ok(map)
    }

    async fn mapgenie_map(&self, slug: &str) -> Result<MarathonMap> {
        let doc = self.mapgenie.map(slug).await?;
        Ok(parse::mapgenie_map_to_model(slug, &doc.taxonomy, &doc.data))
    }

    async fn metaforge_map(&self, slug: &str) -> Result<MarathonMap> {
        let rows = self.metaforge.map_markers(slug).await?;
        if rows.is_empty() {
            return Err(MarathonError::NotFound {
                entity: "map",
                query: slug.to_string(),
            });
        }
        Ok(parse::metaforge_markers_to_map(slug, &rows))
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
            Err(err) => warn!(%err, "marathondb map list unavailable"),
        }

        match self.mapgenie.slugs().await {
            Ok(mg) => mg.into_iter().for_each(&mut push),
            Err(err) => warn!(%err, "mapgenie roster unavailable"),
        }

        if slugs.is_empty() {
            return Err(MarathonError::SourceUnavailable);
        }
        Ok(slugs)
    }
}

fn collect_map(
    candidates: &mut Vec<(SourceId, MarathonMap)>,
    source: SourceId,
    candidate: Result<MarathonMap>,
    slug: &str,
) {
    match candidate {
        Ok(map) => candidates.push((source, map)),
        Err(MarathonError::NotFound { .. }) => {
            debug!(%source, %slug, "source does not cover this map");
        },
        Err(err) => {
            warn!(%source, %slug, %err, "map source unavailable");
        },
    }
}
