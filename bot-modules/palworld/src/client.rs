use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::breeding::BreedingIndex;
use crate::error::{PalworldError, Result};
use crate::model::{Item, Pal, PassiveSkill};
use crate::source::SourceId;
use crate::transport::{Fandom, PalDb, Paldex, PalworldGg};
use crate::{merge, parse};

const LONG_TTL: Duration = Duration::from_hours(8);

fn ttl_cache<K, V>() -> Cache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder().time_to_live(LONG_TTL).max_capacity(512).build()
}

pub struct PalworldClient {
    paldex: Paldex,
    paldb: PalDb,
    palworldgg: PalworldGg,
    fandom: Fandom,

    pal_list_cache: Cache<(), Arc<[Pal]>>,
    pal_cache: Cache<String, Arc<Pal>>,
    item_list_cache: Cache<(), Arc<[Item]>>,
    passive_list_cache: Cache<(), Arc<[PassiveSkill]>>,
    breeding_cache: Cache<(), Arc<BreedingIndex>>,
}

impl PalworldClient {
    #[must_use]
    pub fn new(
        client: Client,
        flaresolverr_url: Option<String>,
        paldex_base: Option<String>,
    ) -> Self {
        Self {
            paldex: Paldex::new(client.clone(), paldex_base),
            paldb: PalDb::new(client.clone(), flaresolverr_url.clone()),
            palworldgg: PalworldGg::new(client.clone(), flaresolverr_url),
            fandom: Fandom::new(client),
            pal_list_cache: ttl_cache(),
            pal_cache: ttl_cache(),
            item_list_cache: ttl_cache(),
            passive_list_cache: ttl_cache(),
            breeding_cache: ttl_cache(),
        }
    }

    pub async fn pals(&self) -> Result<Arc<[Pal]>> {
        if let Some(cached) = self.pal_list_cache.get(&()).await {
            return Ok(cached);
        }
        let raw = self.paldex.pals().await?;
        let pals: Arc<[Pal]> = raw.into_iter().map(parse::pal_from_raw).collect();
        self.pal_list_cache.insert((), Arc::clone(&pals)).await;
        Ok(pals)
    }

    pub async fn pal(&self, key: &str) -> Result<Arc<Pal>> {
        if let Some(cached) = self.pal_cache.get(key).await {
            return Ok(cached);
        }

        let pals = self.pals().await?;
        let base = pals.iter().find(|p| p.key == key).cloned().ok_or_else(|| {
            PalworldError::NotFound { entity: "pal", query: key.to_string() }
        })?;

        let enriched = Arc::new(self.enrich_pal(base).await);
        self.pal_cache.insert(key.to_string(), Arc::clone(&enriched)).await;
        Ok(enriched)
    }

    async fn enrich_pal(&self, base: Pal) -> Pal {
        let name = base.name.clone();
        let key = base.key.clone();

        let (fandom, paldb, palworldgg) = tokio::join!(
            self.fandom.description(&name),
            self.paldb.pal_description(&name),
            self.palworldgg.pal_description(&key),
        );

        let mut candidates = vec![(SourceId::Paldex, base)];
        for (source, desc) in [
            (SourceId::Fandom, fandom),
            (SourceId::PalDb, paldb),
            (SourceId::PalworldGg, palworldgg),
        ] {
            if let Some(description) = desc {
                candidates.push((source, Pal {
                    key: key.clone(),
                    name: name.clone(),
                    description: Some(description),
                    ..Pal::default()
                }));
            }
        }

        merge::pal(&candidates).unwrap_or_else(|| Pal {
            key,
            name,
            ..Pal::default()
        })
    }

    pub async fn items(&self) -> Result<Arc<[Item]>> {
        if let Some(cached) = self.item_list_cache.get(&()).await {
            return Ok(cached);
        }
        let raw = self.paldex.items().await?;
        let items: Arc<[Item]> = raw.into_iter().map(parse::item_from_raw).collect();
        self.item_list_cache.insert((), Arc::clone(&items)).await;
        Ok(items)
    }

    pub async fn item(&self, key: &str) -> Result<Item> {
        let items = self.items().await?;
        let candidates: Vec<(SourceId, Item)> = items
            .iter()
            .filter(|i| i.key == key)
            .map(|i| (SourceId::Paldex, i.clone()))
            .collect();
        merge::item(&candidates).ok_or_else(|| PalworldError::NotFound {
            entity: "item",
            query: key.to_string(),
        })
    }

    pub async fn passives(&self) -> Result<Arc<[PassiveSkill]>> {
        if let Some(cached) = self.passive_list_cache.get(&()).await {
            return Ok(cached);
        }
        let raw = self.paldex.passives().await?;
        let mut passives: Vec<PassiveSkill> = raw
            .into_iter()
            .map(|(key, value)| parse::passive_from_raw(key, value))
            .collect();
        passives.sort_by_key(|p| p.name.to_lowercase());
        let passives: Arc<[PassiveSkill]> = passives.into();
        self.passive_list_cache.insert((), Arc::clone(&passives)).await;
        Ok(passives)
    }

    pub async fn passive(&self, key: &str) -> Result<PassiveSkill> {
        let passives = self.passives().await?;
        let candidates: Vec<(SourceId, PassiveSkill)> = passives
            .iter()
            .filter(|p| p.key == key)
            .map(|p| (SourceId::Paldex, p.clone()))
            .collect();
        merge::passive(&candidates).ok_or_else(|| PalworldError::NotFound {
            entity: "passive skill",
            query: key.to_string(),
        })
    }

    pub async fn breeding_index(&self) -> Result<Arc<BreedingIndex>> {
        if let Some(cached) = self.breeding_cache.get(&()).await {
            return Ok(cached);
        }
        let map = self.paldex.breeding().await?;
        let index = Arc::new(BreedingIndex::from_map(map));
        self.breeding_cache.insert((), Arc::clone(&index)).await;
        Ok(index)
    }
}
