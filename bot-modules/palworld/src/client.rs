use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use moka::future::Cache;
use reqwest::Client;
use tokio::sync::Mutex;

use crate::breeding::BreedingIndex;
use crate::error::{PalworldError, Result};
use crate::model::{Item, Pal, PassiveSkill, WorldRoster};
use crate::source::SourceId;
use crate::transport::{Fandom, PalCalc, PalDb, Paldex, PalworldGg, Pelican};
use crate::{merge, parse, save};

const LONG_TTL: Duration = Duration::from_hours(8);

const SAVE_FRESHNESS: Duration = Duration::from_mins(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKey {
    Shared,
    User(i64),
}

fn ttl_cache<K, V>() -> Cache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder().time_to_live(LONG_TTL).max_capacity(512).build()
}

pub struct PalworldClient {
    palcalc: PalCalc,
    paldex: Paldex,
    paldb: PalDb,
    palworldgg: PalworldGg,
    fandom: Fandom,

    save_dir: Option<PathBuf>,
    uploads_dir: PathBuf,
    pelican: Option<Pelican>,
    refresh_lock: Mutex<Option<Instant>>,

    pal_list_cache: Cache<(), Arc<[Pal]>>,
    pal_cache: Cache<String, Arc<Pal>>,
    item_list_cache: Cache<(), Arc<[Item]>>,
    passive_list_cache: Cache<(), Arc<[PassiveSkill]>>,
    breeding_cache: Cache<(), Arc<BreedingIndex>>,
    roster_cache: Cache<(SourceKey, u64), Arc<WorldRoster>>,
}

impl PalworldClient {
    #[must_use]
    pub fn new(
        client: Client,
        flaresolverr_url: Option<String>,
        paldex_base: Option<String>,
        palcalc_base: Option<String>,
        save_dir: Option<PathBuf>,
        uploads_dir: PathBuf,
        pelican: Option<Pelican>,
    ) -> Self {
        Self {
            palcalc: PalCalc::new(client.clone(), palcalc_base),
            paldex: Paldex::new(client.clone(), paldex_base),
            paldb: PalDb::new(client.clone(), flaresolverr_url.clone()),
            palworldgg: PalworldGg::new(client.clone(), flaresolverr_url),
            fandom: Fandom::new(client),
            save_dir,
            uploads_dir,
            pelican,
            refresh_lock: Mutex::new(None),
            pal_list_cache: ttl_cache(),
            pal_cache: ttl_cache(),
            item_list_cache: ttl_cache(),
            passive_list_cache: ttl_cache(),
            breeding_cache: ttl_cache(),
            roster_cache: ttl_cache(),
        }
    }

    pub async fn pals(&self) -> Result<Arc<[Pal]>> {
        if let Some(cached) = self.pal_list_cache.get(&()).await {
            return Ok(cached);
        }
        let raw = self.palcalc.pals().await?;
        let mut pals: Vec<Pal> =
            raw.into_iter().map(parse::pal_from_palcalc).collect();

        if let Some(index) = self.palworldgg.elements_index().await {
            for pal in &mut pals {
                if let Some(elements) = index.get(&parse::gg_slug(&pal.name)) {
                    pal.elements.clone_from(elements);
                }
            }
        }

        let pals: Arc<[Pal]> = pals.into();
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
        let slug = parse::gg_slug(&name);

        let (fandom, paldb, palworldgg) = tokio::join!(
            self.fandom.description(&name),
            self.paldb.pal_details(&name),
            self.palworldgg.pal_description(&slug),
        );

        let mut candidates = vec![(SourceId::PalCalc, base)];
        candidates.push((SourceId::PalDb, Pal {
            key: key.clone(),
            name: name.clone(),
            description: paldb.description,
            image_url: paldb.image_url,
            ..Pal::default()
        }));
        for (source, desc) in
            [(SourceId::Fandom, fandom), (SourceId::PalworldGg, palworldgg)]
        {
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
        let map = self.palcalc.breeding().await?;
        let index = Arc::new(BreedingIndex::from_map(map));
        self.breeding_cache.insert((), Arc::clone(&index)).await;
        Ok(index)
    }

    pub async fn roster(&self) -> Result<Arc<WorldRoster>> {
        let save_dir = self.save_dir.clone().ok_or_else(|| {
            PalworldError::Save("no save directory configured".to_string())
        })?;
        self.refresh_shared_if_stale().await;
        self.roster_from(SourceKey::Shared, &save_dir).await
    }

    pub async fn user_roster(&self, discord_id: i64) -> Result<Arc<WorldRoster>> {
        let dir = self.uploads_dir.join(discord_id.to_string());
        self.roster_from(SourceKey::User(discord_id), &dir).await
    }

    async fn roster_from(
        &self,
        key: SourceKey,
        dir: &Path,
    ) -> Result<Arc<WorldRoster>> {
        let mtime = std::fs::metadata(dir.join("Level.sav"))?
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .map_or(0, |n| u64::try_from(n).unwrap_or(u64::MAX));

        if let Some(cached) = self.roster_cache.get(&(key, mtime)).await {
            return Ok(cached);
        }

        let load_dir = dir.to_path_buf();
        let roster =
            tokio::task::spawn_blocking(move || save::load_world(&load_dir))
                .await
                .map_err(|e| {
                    PalworldError::Save(format!("save parse task failed: {e}"))
                })?
                .inspect_err(|e| {
                    tracing::error!(
                        error = %e,
                        dir = %dir.display(),
                        "failed to read world save",
                    );
                })?;

        let roster = Arc::new(roster);
        self.roster_cache.insert((key, mtime), Arc::clone(&roster)).await;
        Ok(roster)
    }

    async fn refresh_shared_if_stale(&self) {
        let (Some(save_dir), Some(pelican)) =
            (self.save_dir.clone(), self.pelican.clone())
        else {
            return;
        };

        let mut last = self.refresh_lock.lock().await;
        if last.is_some_and(|t| t.elapsed() < SAVE_FRESHNESS) {
            return;
        }

        if let Err(e) = Self::refresh_shared(&pelican, &save_dir).await {
            tracing::warn!(
                error = %e,
                "palworld shared save refresh failed; using last local save"
            );
        }

        *last = Some(Instant::now());
    }

    async fn refresh_shared(pelican: &Pelican, save_dir: &Path) -> Result<()> {
        let remote_modified = pelican.level_modified().await?;

        let level_path = save_dir.join("Level.sav");
        let local_modified = {
            let level_path = level_path.clone();
            tokio::task::spawn_blocking(move || local_mtime_secs(&level_path))
                .await
                .map_err(|e| {
                    PalworldError::Pelican(format!("mtime task failed: {e}"))
                })?
        };

        if remote_modified <= local_modified {
            return Ok(());
        }

        let bytes = pelican.download_level().await?;

        let save_dir = save_dir.to_path_buf();
        tokio::task::spawn_blocking(move || {
            save::validate_level(&bytes).map_err(|e| {
                PalworldError::Pelican(format!(
                    "downloaded save failed validation, keeping last local save: {e}"
                ))
            })?;
            write_level_atomic(&save_dir, &bytes)
        })
        .await
        .map_err(|e| {
            PalworldError::Pelican(format!("save write task failed: {e}"))
        })??;

        Ok(())
    }
}

fn local_mtime_secs(level_path: &Path) -> i64 {
    std::fs::metadata(level_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn write_level_atomic(save_dir: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(save_dir)?;
    let tmp = save_dir.join("Level.sav.tmp");
    let final_path = save_dir.join("Level.sav");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(())
}
