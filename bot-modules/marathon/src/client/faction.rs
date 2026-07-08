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

        let factions = if let Some(mobalytics) = &self.mobalytics {
            match mobalytics.fetch_document("factions").await {
                Ok(doc) => {
                    let mut factions = Vec::new();
                    for stub in parse::parse_faction_listing(&doc) {
                        factions.push((*self.faction(&stub.slug).await?).clone());
                    }
                    factions
                },
                Err(_) => self.factions_from_marathondb().await?,
            }
        } else {
            self.factions_from_marathondb().await?
        };

        let entry: Arc<[Faction]> = factions.into();
        self.faction_list_cache.insert((), Arc::clone(&entry)).await;
        Ok(entry)
    }

    pub async fn faction(&self, slug: &str) -> Result<Arc<Faction>> {
        if let Some(cached) = self.faction_cache.get(slug).await {
            return Ok(cached);
        }

        let faction = if let Some(mobalytics) = &self.mobalytics {
            match mobalytics.fetch_document(&format!("factions/{slug}")).await {
                Ok(doc) => parse::parse_faction(slug, &doc),
                Err(_) => self.faction_from_marathondb(slug).await?,
            }
        } else {
            self.faction_from_marathondb(slug).await?
        };

        let entry = Arc::new(faction);
        self.faction_cache.insert(slug.to_string(), Arc::clone(&entry)).await;
        Ok(entry)
    }

    async fn factions_from_marathondb(&self) -> Result<Vec<Faction>> {
        let contracts = self.marathondb.contracts().await?;
        Ok(parse::marathondb_contracts_to_factions(&contracts))
    }

    async fn faction_from_marathondb(&self, slug: &str) -> Result<Faction> {
        self.factions_from_marathondb()
            .await?
            .into_iter()
            .find(|f| f.slug == slug)
            .ok_or_else(|| MarathonError::NotFound {
                entity: "faction",
                query: slug.to_string(),
            })
    }
}
