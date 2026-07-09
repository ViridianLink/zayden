use std::sync::Arc;

use super::MarathonClient;
use crate::error::{MarathonError, Result};
use crate::model::Faction;
use crate::parse;

impl MarathonClient {
    pub async fn factions(&self) -> Result<Arc<[Faction]>> {
        if let Some(cached) = self.faction_list_cache.get(&()).await {
            return Ok(cached);
        }

        let factions = match self.marathondb.contracts().await {
            Ok(contracts) => {
                let mut factions =
                    parse::marathondb_contracts_to_factions(&contracts);
                for faction in &mut factions {
                    self.enrich_faction(faction).await;
                }
                factions
            },
            Err(_) => self.factions_from_mobalytics().await?,
        };

        let entry: Arc<[Faction]> = factions.into();
        self.faction_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    pub async fn faction(&self, slug: &str) -> Result<Arc<Faction>> {
        if let Some(cached) = self.faction_cache.get(slug).await {
            return Ok(cached);
        }

        let faction = match self.marathondb.contracts().await {
            Ok(contracts) => {
                let mut faction =
                    parse::marathondb_contracts_to_factions(&contracts)
                        .into_iter()
                        .find(|f| f.slug == slug)
                        .ok_or_else(|| MarathonError::NotFound {
                            entity: "faction",
                            query: slug.to_string(),
                        })?;
                self.enrich_faction(&mut faction).await;
                faction
            },
            // Fall back to Mobalytics only if the structured source is down.
            Err(_) if self.mobalytics.is_some() => {
                self.faction_from_mobalytics(slug).await?
            },
            Err(err) => return Err(err),
        };

        let entry = Arc::new(faction);
        self.faction_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn enrich_faction(&self, faction: &mut Faction) {
        if faction.upgrades.is_empty()
            && let Some(mobalytics) = &self.mobalytics
            && let Ok(doc) = mobalytics
                .fetch_document(&format!("factions/{}", faction.slug))
                .await
        {
            faction.upgrades = parse::parse_faction(&faction.slug, &doc).upgrades;
        }
    }

    async fn factions_from_mobalytics(&self) -> Result<Vec<Faction>> {
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document("factions").await?;

        let mut factions = Vec::new();
        for stub in parse::parse_faction_listing(&doc) {
            match mobalytics.fetch_document(&format!("factions/{}", stub.slug)).await
            {
                Ok(detail) => {
                    factions.push(parse::parse_faction(&stub.slug, &detail));
                },
                Err(_) => factions.push(stub),
            }
        }
        Ok(factions)
    }

    async fn faction_from_mobalytics(&self, slug: &str) -> Result<Faction> {
        let Some(mobalytics) = &self.mobalytics else {
            return Err(MarathonError::SourceUnavailable);
        };
        let doc = mobalytics.fetch_document(&format!("factions/{slug}")).await?;
        Ok(parse::parse_faction(slug, &doc))
    }
}
