use std::sync::Arc;

use tracing::warn;

use super::health::flag_degraded_sources;
use super::{MarathonClient, collect_candidate};
use crate::error::{MarathonError, Result};
use crate::model::Faction;
use crate::source::SourceId;
use crate::{merge, parse};

impl MarathonClient {
    pub async fn factions(&self) -> Result<Arc<[Faction]>> {
        if let Some(cached) = self.faction_list_cache.get(&()).await {
            return Ok(cached);
        }

        let slugs = self.faction_slugs().await?;
        let mut factions = Vec::with_capacity(slugs.len());
        for slug in &slugs {
            factions.push((*self.faction(slug).await?).clone());
        }

        let entry: Arc<[Faction]> = factions.into();
        self.faction_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    pub async fn faction(&self, slug: &str) -> Result<Arc<Faction>> {
        if let Some(cached) = self.faction_cache.get(slug).await {
            return Ok(cached);
        }

        let candidates = self.gather_faction(slug).await;
        let faction = merge::faction(&candidates).ok_or_else(|| {
            MarathonError::NotFound { entity: "faction", query: slug.to_string() }
        })?;

        let entry = Arc::new(faction);
        self.faction_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn gather_faction(&self, slug: &str) -> Vec<(SourceId, Faction)> {
        let (marathondb, mobalytics, cyberacme, tauceti, marathonguide) = tokio::join!(
            self.marathondb_faction(slug),
            self.mobalytics_faction(slug),
            self.cyberacme_faction(slug),
            self.tauceti_faction(slug),
            self.marathonguide_faction(slug),
        );

        let mut out = Vec::new();
        collect_candidate(
            &mut out,
            SourceId::MarathonDb,
            marathondb,
            slug,
            "faction",
        );
        collect_candidate(
            &mut out,
            SourceId::Mobalytics,
            mobalytics,
            slug,
            "faction",
        );
        collect_candidate(&mut out, SourceId::CyberAcme, cyberacme, slug, "faction");
        collect_candidate(&mut out, SourceId::TauCeti, tauceti, slug, "faction");
        collect_candidate(
            &mut out,
            SourceId::MarathonGuide,
            marathonguide,
            slug,
            "faction",
        );
        flag_degraded_sources(&out, "faction", slug);
        out
    }

    async fn marathondb_faction(&self, slug: &str) -> Result<Faction> {
        let contracts = self.marathondb.contracts().await?;
        parse::marathondb_contracts_to_factions(&contracts)
            .into_iter()
            .find(|f| f.slug == slug)
            .ok_or_else(|| MarathonError::NotFound {
                entity: "faction",
                query: slug.to_string(),
            })
    }

    async fn mobalytics_faction(&self, slug: &str) -> Result<Faction> {
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document(&format!("factions/{slug}")).await?;
        Ok(parse::parse_faction(slug, &doc))
    }

    async fn cyberacme_faction(&self, slug: &str) -> Result<Faction> {
        let envelope = self.cyberacme.faction(slug).await?;
        Ok(parse::cyberacme_faction_to_model(slug, &envelope))
    }

    async fn tauceti_faction(&self, slug: &str) -> Result<Faction> {
        let Some(tauceti) = &self.tauceti else {
            return Err(MarathonError::SourceUnavailable);
        };
        let value = tauceti.faction(slug).await?;
        Ok(parse::tauceti_faction_to_model(slug, &value))
    }

    async fn marathonguide_faction(&self, slug: &str) -> Result<Faction> {
        let (contracts, upgrades) = tokio::join!(
            self.marathonguide.faction_contracts(slug),
            self.marathonguide.faction_upgrades(slug),
        );

        let contracts = contracts
            .inspect_err(|err| {
                warn!(
                    %err,
                    slug,
                    "marathon-guide faction contracts page unavailable"
                );
            })
            .ok();
        let upgrades = upgrades
            .inspect_err(|err| {
                warn!(
                    %err,
                    slug,
                    "marathon-guide faction upgrades page unavailable"
                );
            })
            .ok();

        if contracts.is_none() && upgrades.is_none() {
            return Err(MarathonError::NotFound {
                entity: "faction",
                query: slug.to_string(),
            });
        }

        Ok(parse::marathonguide_html_to_faction(
            slug,
            contracts.as_deref(),
            upgrades.as_deref(),
        ))
    }

    async fn faction_slugs(&self) -> Result<Vec<String>> {
        let mut slugs: Vec<String> = Vec::new();
        let mut push = |slug: String| {
            if !slugs.contains(&slug) {
                slugs.push(slug);
            }
        };

        match self.marathondb.contracts().await {
            Ok(contracts) => parse::marathondb_contracts_to_factions(&contracts)
                .into_iter()
                .for_each(|f| push(f.slug)),
            Err(err) => warn!(%err, "marathondb faction list unavailable"),
        }

        if let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) = mobalytics.fetch_document("factions").await
        {
            for f in parse::parse_faction_listing(&doc) {
                push(f.slug);
            }
        }

        match self.cyberacme.factions().await {
            Ok(factions) => factions
                .iter()
                .filter_map(|f| {
                    f.get("slug")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .for_each(&mut push),
            Err(err) => warn!(%err, "cyberacme faction list unavailable"),
        }

        if slugs.is_empty() {
            return Err(MarathonError::SourceUnavailable);
        }
        Ok(slugs)
    }
}
