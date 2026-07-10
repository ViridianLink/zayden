use std::sync::Arc;

use serde_json::Value;

use super::{MarathonClient, collect_candidate};
use crate::error::{MarathonError, Result};
use crate::model::Runner;
use crate::source::SourceId;
use crate::{merge, parse};

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
        let candidates = self.gather_runner(slug).await;
        let mut runner = merge::runner(&candidates).ok_or_else(|| {
            MarathonError::NotFound { entity: "runner", query: slug.to_string() }
        })?;

        if runner.description.is_none() {
            runner.description = self.fandom.description(&runner.name).await;
        }
        Ok(runner)
    }

    async fn gather_runner(&self, slug: &str) -> Vec<(SourceId, Runner)> {
        let (
            marathondb,
            mobalytics,
            cyberacme,
            tauceti,
            marathonmeta,
            marathonguide,
        ) = tokio::join!(
            self.marathondb_runner(slug),
            self.mobalytics_runner(slug),
            self.cyberacme_runner(slug),
            self.tauceti_runner(slug),
            self.marathonmeta_runner(slug),
            self.marathonguide_runner(slug),
        );

        let mut out = Vec::new();
        collect_candidate(
            &mut out,
            SourceId::MarathonDb,
            marathondb,
            slug,
            "runner",
        );
        collect_candidate(
            &mut out,
            SourceId::Mobalytics,
            mobalytics,
            slug,
            "runner",
        );
        collect_candidate(&mut out, SourceId::CyberAcme, cyberacme, slug, "runner");
        collect_candidate(&mut out, SourceId::TauCeti, tauceti, slug, "runner");
        collect_candidate(
            &mut out,
            SourceId::MarathonMeta,
            marathonmeta,
            slug,
            "runner",
        );
        collect_candidate(
            &mut out,
            SourceId::MarathonGuide,
            marathonguide,
            slug,
            "runner",
        );
        out
    }

    async fn marathondb_runner(&self, slug: &str) -> Result<Runner> {
        let data = self.marathondb.runner(slug).await?;
        Ok(parse::marathondb_runner_to_model(slug, &data))
    }

    async fn mobalytics_runner(&self, slug: &str) -> Result<Runner> {
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document(&format!("runners/{slug}")).await?;
        Ok(parse::parse_runner(slug, &doc))
    }

    async fn cyberacme_runner(&self, slug: &str) -> Result<Runner> {
        let runner = self.cyberacme.runner(slug).await?;
        Ok(parse::cyberacme_runner_to_model(slug, &runner))
    }

    async fn tauceti_runner(&self, slug: &str) -> Result<Runner> {
        let Some(tauceti) = &self.tauceti else {
            return Err(MarathonError::SourceUnavailable);
        };
        let runner = tauceti.runner(slug).await?;
        Ok(parse::tauceti_runner_to_model(slug, &runner))
    }

    async fn marathonmeta_runner(&self, slug: &str) -> Result<Runner> {
        let Some(marathonmeta) = &self.marathonmeta else {
            return Err(MarathonError::SourceUnavailable);
        };
        let rendered = marathonmeta.runner(slug).await?;
        Ok(parse::marathonmeta_html_to_runner(slug, &rendered))
    }

    async fn marathonguide_runner(&self, slug: &str) -> Result<Runner> {
        let page = self.marathonguide.runner(slug).await?;
        Ok(parse::marathonguide_html_to_runner(slug, &page))
    }

    pub async fn runners(&self) -> Result<Arc<[Runner]>> {
        if let Some(cached) = self.runner_list_cache.get(&()).await {
            return Ok(cached);
        }

        let slugs = self.runner_slugs().await?;
        let mut runners = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            runners.push((*self.runner(slug).await?).clone());
        }

        let entry: Arc<[Runner]> = runners.into();
        self.runner_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn runner_slugs(&self) -> Result<Vec<String>> {
        match self.marathondb.runners().await {
            Ok(items) => Ok(items
                .iter()
                .filter_map(|r| {
                    r.get("slug").and_then(Value::as_str).map(str::to_string)
                })
                .collect()),
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    mobalytics.fetch_listing_slugs("runners", "runners").await
                },
                None => Err(err),
            },
        }
    }
}
