use std::sync::Arc;

use serde_json::Value;

use super::MarathonClient;
use crate::error::Result;
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
        match self.marathondb.runner(slug).await {
            Ok(data) => {
                let mut runner = parse::marathondb_runner_to_model(slug, &data);
                self.enrich_runner(&mut runner).await;
                Ok(runner)
            },
            Err(err) => match &self.mobalytics {
                Some(mobalytics) => {
                    let doc = mobalytics
                        .fetch_document(&format!("runners/{slug}"))
                        .await?;
                    Ok(parse::parse_runner(slug, &doc))
                },
                None => Err(err),
            },
        }
    }

    async fn enrich_runner(&self, runner: &mut Runner) {
        if runner.cores.is_empty()
            && let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) =
                mobalytics.fetch_document(&format!("runners/{}", runner.slug)).await
        {
            let parsed = parse::parse_runner(&runner.slug, &doc);
            runner.cores = parsed.cores;
            if runner.portrait_url.is_none() {
                runner.portrait_url = parsed.portrait_url;
            }
        }

        if runner.description.is_none() {
            runner.description = self.fandom.description(&runner.name).await;
        }
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
