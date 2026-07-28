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
use crate::save::player::PlayerRecord;
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
    record_cache: Cache<(SourceKey, String, u64), Arc<PlayerRecord>>,
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
            record_cache: ttl_cache(),
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
        let save_dir = self.save_dir.clone().ok_or(PalworldError::NoWorld)?;
        self.refresh_shared_if_stale().await;
        self.roster_from(SourceKey::Shared, &save_dir).await
    }

    pub async fn user_roster(&self, discord_id: i64) -> Result<Arc<WorldRoster>> {
        self.roster_from(SourceKey::User(discord_id), &self.user_dir(discord_id))
            .await
    }

    pub async fn player_record(
        &self,
        source: SourceKey,
        uid: &str,
    ) -> Result<Option<Arc<PlayerRecord>>> {
        let dir = match source {
            SourceKey::Shared => {
                self.refresh_shared_if_stale().await;
                self.save_dir.clone().ok_or(PalworldError::NoWorld)?
            },
            SourceKey::User(discord_id) => self.user_dir(discord_id),
        };

        let Some(path) = save::player_save_path(&dir, uid) else {
            return Ok(None);
        };
        let Some(mtime) = file_mtime(&path) else { return Ok(None) };

        let key = (source, uid.to_string(), mtime);
        if let Some(cached) = self.record_cache.get(&key).await {
            return Ok(Some(cached));
        }

        let uid_owned = uid.to_string();
        let record = tokio::task::spawn_blocking(move || {
            save::player::load_player(&dir, &uid_owned)
        })
        .await
        .map_err(|e| {
            PalworldError::Save(format!("player save parse task failed: {e}"))
        })??;

        let Some(record) = record else { return Ok(None) };
        let record = Arc::new(record);
        self.record_cache.insert(key, Arc::clone(&record)).await;
        Ok(Some(record))
    }

    fn user_dir(&self, discord_id: i64) -> PathBuf {
        self.uploads_dir.join(discord_id.to_string())
    }

    #[must_use]
    pub fn uploads_dir(&self) -> &Path {
        &self.uploads_dir
    }

    async fn roster_from(
        &self,
        key: SourceKey,
        dir: &Path,
    ) -> Result<Arc<WorldRoster>> {
        let mtime = mtime_nanos(&std::fs::metadata(dir.join("Level.sav"))?);

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
        let level = Self::refresh_shared_level(pelican, save_dir).await;

        if let Err(e) = Self::refresh_shared_players(pelican, save_dir).await {
            tracing::warn!(
                error = %e,
                "palworld: player save refresh failed; using last local copies"
            );
        }

        level
    }

    async fn refresh_shared_level(pelican: &Pelican, save_dir: &Path) -> Result<()> {
        let remote_modified = pelican.level_modified().await?;

        let level_path = save_dir.join("Level.sav");
        let local_modified =
            tokio::task::spawn_blocking(move || local_mtime_secs(&level_path))
                .await
                .map_err(|e| {
                    PalworldError::Pelican(format!("mtime task failed: {e}"))
                })?;

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
            save::write_level_atomic(&save_dir, &bytes)
        })
        .await
        .map_err(|e| {
            PalworldError::Pelican(format!("save write task failed: {e}"))
        })??;

        Ok(())
    }

    async fn refresh_shared_players(
        pelican: &Pelican,
        save_dir: &Path,
    ) -> Result<()> {
        let players_dir = save_dir.join("Players");

        for remote in pelican.list_players().await? {
            let local = players_dir.join(format!("{}.sav", remote.stem));
            let local_modified =
                tokio::task::spawn_blocking(move || local_mtime_secs(&local))
                    .await
                    .map_err(|e| {
                        PalworldError::Pelican(format!("mtime task failed: {e}"))
                    })?;
            if remote.modified <= local_modified {
                continue;
            }

            let bytes = match pelican.download_player(&remote.stem).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        player = remote.stem,
                        "palworld: player save download failed"
                    );
                    continue;
                },
            };

            let save_dir = save_dir.to_path_buf();
            let stem = remote.stem.clone();
            let is_storage = remote.is_storage;
            let written = tokio::task::spawn_blocking(move || {
                validate_player_file(&bytes, is_storage).map_err(|e| {
                    PalworldError::Pelican(format!(
                        "downloaded player save failed validation: {e}"
                    ))
                })?;
                save::write_raw_player(&save_dir, &stem, &bytes)
            })
            .await
            .map_err(|e| {
                PalworldError::Pelican(format!("save write task failed: {e}"))
            })?;

            if let Err(e) = written {
                tracing::warn!(
                    error = %e,
                    player = remote.stem,
                    "palworld: keeping last local player save"
                );
            }
        }

        Ok(())
    }
}

fn validate_player_file(bytes: &[u8], is_storage: bool) -> Result<()> {
    if is_storage {
        save::dps::parse(bytes).map(|_| ())
    } else {
        save::player::parse_player_uid(bytes).map(|_| ())
    }
}

fn local_mtime_secs(level_path: &Path) -> i64 {
    std::fs::metadata(level_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn mtime_nanos(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

fn file_mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| mtime_nanos(&m))
}
